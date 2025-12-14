use anchor_lang::{
    prelude::*,
    solana_program::{
        program::{invoke, invoke_signed},
        system_instruction::transfer,
    },
};
use anchor_spl::{
    associated_token::{create, AssociatedToken},
    token::Token,
    token_2022::{self, Token2022},
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use magnus_consts::spl_token;

use crate::error::ErrorCode;

pub fn transfer_token<'a>(
    authority: AccountInfo<'a>,
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    amount: u64,
    mint_decimals: u8,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    if let Some(signer_seeds) = signer_seeds {
        token_2022::transfer_checked(
            CpiContext::new_with_signer(token_program.to_account_info(), token_2022::TransferChecked { from, to, authority, mint }, signer_seeds),
            amount,
            mint_decimals,
        )
    } else {
        token_2022::transfer_checked(CpiContext::new(token_program.to_account_info(), token_2022::TransferChecked { from, to, authority, mint }), amount, mint_decimals)
    }
}

pub fn transfer_sol<'a>(from: AccountInfo<'a>, to: AccountInfo<'a>, lamports: u64, signer_seeds: Option<&[&[&[u8]]]>) -> Result<()> {
    if lamports == 0 {
        return Ok(());
    }
    let ix = transfer(from.key, to.key, lamports);
    if let Some(signer_seeds) = signer_seeds {
        invoke_signed(&ix, &[from, to], signer_seeds)?;
    } else {
        invoke(&ix, &[from, to])?;
    }
    Ok(())
}

pub fn close_token_account<'a>(
    token_account: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    if token_account.get_lamports() == 0 {
        return Ok(());
    }
    if let Some(signer_seeds) = signer_seeds {
        token_2022::close_account(CpiContext::new_with_signer(
            token_program.to_account_info(),
            token_2022::CloseAccount { account: token_account, destination, authority },
            signer_seeds,
        ))
    } else {
        token_2022::close_account(CpiContext::new(token_program.to_account_info(), token_2022::CloseAccount { account: token_account, destination, authority }))
    }
}

pub fn create_sa_if_needed<'info>(
    payer: &AccountInfo<'info>,
    mint: &InterfaceAccount<'info, Mint>,
    sa_authority: &Option<UncheckedAccount<'info>>,
    token_sa: &mut Option<UncheckedAccount<'info>>,
    token_program: &Option<Interface<'info, TokenInterface>>,
    associated_token_program: &Option<Program<'info, AssociatedToken>>,
    system_program: &Option<Program<'info, System>>,
) -> Result<Option<InterfaceAccount<'info, TokenAccount>>> {
    if sa_authority.is_none() || token_sa.is_none() || token_program.is_none() || associated_token_program.is_none() || system_program.is_none() {
        return Ok(None);
    }
    let sa_authority = sa_authority.as_ref().unwrap();
    let token_sa = token_sa.as_ref().unwrap();
    let associated_token_program = associated_token_program.as_ref().unwrap();
    let system_program = system_program.as_ref().unwrap();
    let token_program = token_program.as_ref().unwrap();

    if !is_token_account_initialized(token_sa) {
        create(CpiContext::new(
            associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: payer.to_account_info(),
                associated_token: token_sa.to_account_info(),
                authority: sa_authority.to_account_info(),
                mint: mint.to_account_info(),
                system_program: system_program.to_account_info(),
                token_program: token_program.to_account_info(),
            },
        ))?;
    }
    let token_sa_box = Box::leak(Box::new(token_sa.clone()));
    Ok(Some(InterfaceAccount::<TokenAccount>::try_from(token_sa_box)?))
}

/// Check if the token account is initialized
pub fn is_token_account_initialized(account: &AccountInfo) -> bool {
    // Check if the account has been rented (has allocated space) or is empty
    if account.lamports() == 0 || account.data_is_empty() {
        return false;
    }
    // Check if the account owner is the Token program
    if *account.owner != Token::id() && *account.owner != Token2022::id() {
        return false;
    }
    true
}

pub fn associate_convert_token_account<'info>(token_account: &AccountInfo<'info>) -> Result<InterfaceAccount<'info, TokenAccount>> {
    let account_box = Box::leak(Box::new(token_account.as_ref().to_account_info()));
    InterfaceAccount::<TokenAccount>::try_from(account_box).map_err(|_| ErrorCode::InvalidTokenAccount.into())
}

pub fn is_ata(account: &AccountInfo) -> bool {
    account.as_ref().owner == &spl_token::ID || account.as_ref().owner == &crate::token_2022_program::ID
}

pub fn is_system_account(account: &AccountInfo) -> bool {
    account.as_ref().owner == &crate::system_program::ID
}
