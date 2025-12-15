use crate::constants::*;
use crate::error::ErrorCode;
use crate::utils::*;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

// calculate commission and platform fee amount
pub fn calculate_fee_amounts(amount: u64, commission_rate: u32, commission_direction: bool, platform_fee_rate: Option<u16>) -> Result<(u64, u64)> {
    if commission_rate == 0 {
        return Ok((0, 0));
    }
    require!(commission_rate <= COMMISSION_RATE_LIMIT_V2, ErrorCode::InvalidCommissionRate);

    let commission_amount = if commission_direction {
        u64::try_from(
            u128::from(amount)
                .checked_mul(commission_rate as u128)
                .ok_or(ErrorCode::CalculationError)?
                .checked_div(COMMISSION_DENOMINATOR_V2 as u128 - commission_rate as u128)
                .ok_or(ErrorCode::CalculationError)?,
        )
        .unwrap()
    } else {
        u64::try_from(
            u128::from(amount)
                .checked_mul(commission_rate as u128)
                .ok_or(ErrorCode::CalculationError)?
                .checked_div(COMMISSION_DENOMINATOR_V2 as u128)
                .ok_or(ErrorCode::CalculationError)?,
        )
        .unwrap()
    };

    let platform_fee_amount = if platform_fee_rate.is_some() && platform_fee_rate.unwrap() > 0 {
        let platform_fee_rate = platform_fee_rate.unwrap();
        require!(platform_fee_rate as u64 <= PLATFORM_FEE_RATE_LIMIT_V3, ErrorCode::InvalidPlatformFeeRate);
        u64::try_from(
            u128::from(commission_amount)
                .checked_mul(platform_fee_rate as u128)
                .ok_or(ErrorCode::CalculationError)?
                .checked_div(PLATFORM_FEE_DENOMINATOR_V3 as u128)
                .ok_or(ErrorCode::CalculationError)?,
        )
        .unwrap()
    } else {
        0
    };
    require!(platform_fee_amount <= commission_amount, ErrorCode::InvalidPlatformFeeAmount);

    // commission_amount - platform_fee_amount
    let commission_amount = commission_amount.checked_sub(platform_fee_amount).ok_or(ErrorCode::CalculationError)?;

    Ok((commission_amount, platform_fee_amount))
}

// calculate trim amount
pub fn calculate_trim_amount(
    amount: u64,
    expected_amount_out: u64,
    commission_amount: u64,
    platform_fee_amount: u64,
    commission_direction: bool,
    trim_rate: Option<u8>,
) -> Result<u64> {
    if trim_rate.is_none() || trim_rate.unwrap() == 0 {
        return Ok(0);
    }
    let trim_rate = trim_rate.unwrap();
    require!(trim_rate <= TRIM_RATE_LIMIT_V2, ErrorCode::InvalidTrimRate);

    let trim_limit = u64::try_from(u128::from(amount).saturating_mul(trim_rate as u128).saturating_div(TRIM_DENOMINATOR_V2 as u128)).unwrap();

    let trim_amount = if commission_direction {
        (amount.saturating_sub(expected_amount_out)).min(trim_limit)
    } else {
        (amount.saturating_sub(commission_amount).saturating_sub(platform_fee_amount).saturating_sub(expected_amount_out)).min(trim_limit)
    };
    Ok(trim_amount)
}

pub fn transfer_token_fee<'a>(
    authority: &AccountInfo<'a>,
    token_account: &AccountInfo<'a>,
    token_mint: &InterfaceAccount<'a, Mint>,
    token_program: &Interface<'a, TokenInterface>,
    fee_account: &AccountInfo<'a>,
    fee_amount: u64,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    if fee_amount == 0 {
        return Ok(());
    }
    let fee_to_token_account = associate_convert_token_account(fee_account)?;
    require!(fee_to_token_account.mint == token_mint.key(), ErrorCode::InvalidFeeTokenAccount);
    transfer_token(
        authority.to_account_info(),
        token_account.to_account_info(),
        fee_to_token_account.to_account_info(),
        token_mint.to_account_info(),
        token_program.to_account_info(),
        fee_amount,
        token_mint.decimals,
        signer_seeds,
    )
}

pub fn transfer_sol_fee<'a>(authority: &AccountInfo<'a>, fee_account: &AccountInfo<'a>, fee_amount: u64, signer_seeds: Option<&[&[&[u8]]]>) -> Result<u64> {
    if fee_amount == 0 {
        return Ok(0);
    }
    require!(fee_account.owner == &anchor_lang::system_program::ID, ErrorCode::InvalidFeeAccount);

    let actual_fee_amount = if authority.key() != authority_pda::id() {
        let before_sol_balance = fee_account.lamports();
        let after_sol_balance = before_sol_balance.checked_add(fee_amount).ok_or(ErrorCode::CalculationError)?;
        if after_sol_balance < MIN_SOL_ACCOUNT_RENT {
            MIN_SOL_ACCOUNT_RENT.checked_sub(before_sol_balance).ok_or(ErrorCode::CalculationError)?
        } else {
            fee_amount
        }
    } else {
        fee_amount
    };

    transfer_sol(authority.to_account_info(), fee_account.to_account_info(), actual_fee_amount, signer_seeds)?;
    Ok(actual_fee_amount)
}

pub fn is_charge_sol(commission_account: &Option<AccountInfo>, platform_fee_account: &Option<AccountInfo>, token_mint: &InterfaceAccount<Mint>) -> bool {
    if token_mint.key() != wsol_program::ID {
        return false;
    }
    if commission_account.is_some() && commission_account.as_ref().unwrap().owner == &anchor_lang::system_program::ID {
        return true;
    }
    if platform_fee_account.is_some() && platform_fee_account.as_ref().unwrap().owner == &anchor_lang::system_program::ID {
        return true;
    }
    false
}
