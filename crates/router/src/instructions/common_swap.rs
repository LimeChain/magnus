use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{adapters::*, constants::*, error::ErrorCode, processor::*, utils::*};

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum Dex {
    RaydiumClV2,
    RaydiumCp,
    ObricV2,
    SolfiV2,
    Zerofi,
    Humidifi,
}

#[derive(Debug)]
pub struct HopAccounts {
    pub last_to_account: Pubkey,
    pub from_account: Pubkey,
    pub to_account: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Route {
    pub dexes: Vec<Dex>,
    pub weights: Vec<u8>,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct SwapArgs {
    pub amount_in: u64,
    pub expect_amount_out: u64,
    pub min_return: u64,
    pub amounts: Vec<u64>,       // 1st level split amount
    pub routes: Vec<Vec<Route>>, // 2nd level split route
}

#[event]
#[derive(Debug)]
pub struct SwapEvent {
    pub dex: Dex,
    pub amount_in: u64,
    pub amount_out: u64,
}

pub fn common_swap<'info, T: CommonSwapProcessor<'info>>(
    swap_processor: &T,
    payer: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    owner_seeds: Option<&[&[&[u8]]]>,
    source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    source_mint: &InterfaceAccount<'info, Mint>,
    destination_mint: &InterfaceAccount<'info, Mint>,
    sa_authority: &Option<UncheckedAccount<'info>>,
    source_token_sa: &mut Option<UncheckedAccount<'info>>,
    destination_token_sa: &mut Option<UncheckedAccount<'info>>,
    source_token_program: &Option<Interface<'info, TokenInterface>>,
    destination_token_program: &Option<Interface<'info, TokenInterface>>,
    associated_token_program: &Option<Program<'info, AssociatedToken>>,
    system_program: &Option<Program<'info, System>>,
    remaining_accounts: &'info [AccountInfo<'info>],
    args: SwapArgs,
    order_id: u64,
    fee_rate: Option<u32>,
    fee_direction: Option<bool>,
    fee_token_account: Option<&InterfaceAccount<'info, TokenAccount>>,
) -> Result<u64> {
    log_swap_basic_info(order_id, &source_mint.key(), &destination_mint.key(), &source_token_account.owner, &destination_token_account.owner);

    let before_source_balance = source_token_account.amount;
    let before_destination_balance = destination_token_account.amount;
    let min_return = args.min_return;

    log_swap_balance_before(before_source_balance, before_destination_balance, args.amount_in, args.expect_amount_out, min_return);

    // Verify sa_authority is valid
    if sa_authority.is_some() {
        require!(sa_authority.as_ref().unwrap().key() == authority_pda::ID, ErrorCode::InvalidSaAuthority);
    }

    // get swap accounts
    let (mut source_account, mut destination_account) = swap_processor.get_swap_accounts(
        payer,
        source_token_account,
        destination_token_account,
        source_mint,
        destination_mint,
        sa_authority,
        source_token_sa,
        destination_token_sa,
        source_token_program,
        destination_token_program,
        associated_token_program,
        system_program,
    )?;

    // before swap hook
    let real_amount_in = swap_processor.before_swap(
        owner,
        source_token_account,
        source_mint,
        source_token_sa,
        source_token_program,
        args.amount_in,
        owner_seeds,
        fee_rate,
        fee_direction,
        fee_token_account,
    )?;

    // Common swap
    let amount_out = execute_swap(&mut source_account, &mut destination_account, remaining_accounts, args, real_amount_in, source_token_sa.is_some(), owner_seeds)?;

    // after swap hook
    swap_processor.after_swap(
        sa_authority,
        destination_token_account,
        destination_mint,
        destination_token_sa,
        destination_token_program,
        amount_out,
        Some(SA_AUTHORITY_SEED),
        fee_rate,
        fee_direction,
        fee_token_account,
    )?;

    // source token account has been closed in pumpfun buy
    let after_source_balance = if source_token_account.get_lamports() != 0 {
        source_token_account.reload()?;
        source_token_account.amount
    } else {
        0
    };

    let source_token_change = before_source_balance.checked_sub(after_source_balance).ok_or(ErrorCode::CalculationError)?;

    destination_token_account.reload()?;
    let after_destination_balance = destination_token_account.amount;
    let destination_token_change = after_destination_balance.checked_sub(before_destination_balance).ok_or(ErrorCode::CalculationError)?;

    log_swap_end(after_source_balance, after_destination_balance, source_token_change, destination_token_change);

    // Check min return
    require!(destination_token_change >= min_return, ErrorCode::MinReturnNotReached);
    Ok(destination_token_change)
}

pub fn common_swap_v3<'info, T: PlatformFeeV3Processor<'info>>(
    swap_processor: &T,
    payer: &AccountInfo<'info>,
    source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    source_mint: &InterfaceAccount<'info, Mint>,
    destination_mint: &InterfaceAccount<'info, Mint>,
    sa_authority: &Option<UncheckedAccount<'info>>,
    source_token_sa: &mut Option<UncheckedAccount<'info>>,
    destination_token_sa: &mut Option<UncheckedAccount<'info>>,
    source_token_program: &Option<Interface<'info, TokenInterface>>,
    destination_token_program: &Option<Interface<'info, TokenInterface>>,
    associated_token_program: &Option<Program<'info, AssociatedToken>>,
    system_program: &Option<Program<'info, System>>,
    remaining_accounts: &'info [AccountInfo<'info>],
    args: SwapArgs,
    order_id: u64,
    commission_rate: u32,
    commission_direction: bool,
    commission_account: &Option<AccountInfo<'info>>,
    platform_fee_rate: Option<u16>,
    platform_fee_account: &Option<AccountInfo<'info>>,
    trim_rate: Option<u8>,
    trim_account: Option<&AccountInfo<'info>>,
    acc_close_flag: bool,
) -> Result<u64> {
    log_swap_basic_info(order_id, &source_mint.key(), &destination_mint.key(), &source_token_account.owner, &destination_token_account.owner);

    let before_source_balance = source_token_account.amount;
    let before_destination_balance = destination_token_account.amount;
    let min_return = args.min_return;

    log_swap_balance_before(before_source_balance, before_destination_balance, args.amount_in, args.expect_amount_out, min_return);

    // Verify sa_authority is valid
    if sa_authority.is_some() {
        require!(sa_authority.as_ref().unwrap().key() == authority_pda::ID, ErrorCode::InvalidSaAuthority);
    }

    // get swap accounts
    let (mut source_account, mut destination_account) = swap_processor.get_swap_accounts(
        payer,
        source_token_account,
        destination_token_account,
        source_mint,
        destination_mint,
        sa_authority,
        source_token_sa,
        destination_token_sa,
        source_token_program,
        destination_token_program,
        associated_token_program,
        system_program,
    )?;

    // before swap hook
    let real_amount_in = swap_processor.before_swap(
        payer,
        sa_authority,
        source_token_account,
        source_mint,
        source_token_sa,
        source_token_program,
        args.amount_in,
        commission_rate,
        commission_direction,
        commission_account,
        platform_fee_rate,
        platform_fee_account,
    )?;

    // Common swap
    let expected_amount_out = args.expect_amount_out;
    let amount_out = execute_swap(&mut source_account, &mut destination_account, remaining_accounts, args, real_amount_in, source_token_sa.is_some(), None)?;

    // after swap hook
    let actual_amount_out = swap_processor.after_swap(
        payer,
        sa_authority,
        destination_token_account,
        destination_mint,
        destination_token_sa,
        destination_token_program,
        expected_amount_out,
        amount_out,
        commission_rate,
        commission_direction,
        commission_account,
        platform_fee_rate,
        platform_fee_account,
        trim_rate,
        trim_account,
        acc_close_flag,
    )?;

    // source token account has been closed in pumpfun buy
    let after_source_balance = if source_token_account.get_lamports() != 0 {
        source_token_account.reload()?;
        source_token_account.amount
    } else {
        0
    };
    let source_token_change = before_source_balance.checked_sub(after_source_balance).ok_or(ErrorCode::CalculationError)?;

    // destination token account has been closed in swap_tob_processor
    let (after_destination_balance, destination_token_change) = if destination_token_account.get_lamports() != 0 {
        destination_token_account.reload()?;
        let after_destination_balance = destination_token_account.amount;
        (after_destination_balance, after_destination_balance.checked_sub(before_destination_balance).ok_or(ErrorCode::CalculationError)?)
    } else {
        (actual_amount_out, actual_amount_out)
    };

    log_swap_end(after_source_balance, after_destination_balance, source_token_change, destination_token_change);

    // Check min return
    require!(destination_token_change >= min_return, ErrorCode::MinReturnNotReached);
    Ok(destination_token_change)
}

fn execute_swap<'info>(
    source_account: &mut InterfaceAccount<'info, TokenAccount>,
    destination_account: &mut InterfaceAccount<'info, TokenAccount>,
    remaining_accounts: &'info [AccountInfo<'info>],
    args: SwapArgs,
    real_amount_in: u64,
    proxy_from: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    destination_account.reload()?;
    let before_destination_balance = destination_account.amount;

    // Check SwapArgs
    let SwapArgs { amount_in: _, min_return, expect_amount_out, amounts, routes } = &args;
    require!(real_amount_in > 0, ErrorCode::AmountInMustBeGreaterThanZero);
    require!(*min_return > 0, ErrorCode::MinReturnMustBeGreaterThanZero);
    require!(*expect_amount_out >= *min_return, ErrorCode::InvalidExpectAmountOut);
    require!(amounts.len() == routes.len(), ErrorCode::AmountsAndRoutesMustHaveTheSameLength);

    let total_amounts: u64 = amounts.iter().try_fold(0u64, |acc, &x| acc.checked_add(x).ok_or(ErrorCode::CalculationError))?;
    require!(total_amounts == real_amount_in, ErrorCode::TotalAmountsMustBeEqualToAmountIn);

    // Swap by Routes
    let mut offset: usize = 0;
    // Level 1 split handling
    for (i, hops) in routes.iter().enumerate() {
        require!(hops.len() <= MAX_HOPS, ErrorCode::TooManyHops);
        let mut amount_in = amounts[i];

        // Multi-hop handling
        let mut last_to_account = ZERO_ADDRESS;
        for (hop, route) in hops.iter().enumerate() {
            let dexes = &route.dexes;
            let weights = &route.weights;
            require!(dexes.len() == weights.len(), ErrorCode::DexesAndWeightsMustHaveTheSameLength);
            let total_weight: u8 = weights.iter().try_fold(0u8, |acc, &x| acc.checked_add(x).ok_or(ErrorCode::CalculationError))?;
            require!(total_weight == TOTAL_WEIGHT, ErrorCode::WeightsMustSumTo100);

            // Level 2 split handling
            let mut hop_accounts = HopAccounts { last_to_account, from_account: ZERO_ADDRESS, to_account: ZERO_ADDRESS };
            let mut amount_out: u64 = 0;
            let mut acc_fork_in: u64 = 0;
            for (index, dex) in dexes.iter().enumerate() {
                // Calculate 2 level split amount
                let fork_amount_in = if index == dexes.len() - 1 {
                    // The last dex, use the remaining amount_in for trading to prevent accumulation
                    amount_in.checked_sub(acc_fork_in).ok_or(ErrorCode::CalculationError)?
                } else {
                    let temp_amount =
                        amount_in.checked_mul(weights[index] as u64).ok_or(ErrorCode::CalculationError)?.checked_div(TOTAL_WEIGHT as u64).ok_or(ErrorCode::CalculationError)?;
                    acc_fork_in = acc_fork_in.checked_add(temp_amount).ok_or(ErrorCode::CalculationError)?;
                    temp_amount
                };

                // Execute swap
                let fork_amount_out = distribute_swap(dex, remaining_accounts, fork_amount_in, &mut offset, &mut hop_accounts, hop, proxy_from, owner_seeds)?;

                // Emit SwapEvent
                let event = SwapEvent { dex: *dex, amount_in: fork_amount_in, amount_out: fork_amount_out };
                emit!(event);
                msg!("{:?}", event);
                hop_accounts.from_account.log();
                hop_accounts.to_account.log();

                amount_out = amount_out.checked_add(fork_amount_out).ok_or(ErrorCode::CalculationError)?;
            }

            if hop == 0 {
                // CHECK: Verify the first hop's from_token must be consistent with ctx.accounts.source_token_account
                require!(source_account.key() == hop_accounts.from_account, ErrorCode::InvalidSourceTokenAccount);
            }
            if hop == hops.len() - 1 {
                // CHECK: Verify the last hop's to_account must be consistent with ctx.accounts.destination_token_account
                require!(destination_account.key() == hop_accounts.to_account, ErrorCode::InvalidDestinationTokenAccount);
            }
            amount_in = amount_out;
            last_to_account = hop_accounts.to_account;
        }
    }

    destination_account.reload()?;
    let after_destination_balance = destination_account.amount;
    let amount_out = after_destination_balance.checked_sub(before_destination_balance).ok_or(ErrorCode::CalculationError)?;
    Ok(amount_out)
}

fn distribute_swap<'a>(
    dex: &Dex,
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_from: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    let swap_function = match dex {
        Dex::RaydiumClV2 => raydium_cl_v2::swap,
        Dex::RaydiumCp => raydium_cp::swap,
        Dex::ObricV2 => obric_v2::swap,
        Dex::Zerofi => zerofi::swap,
        Dex::Humidifi => humidifi::swap,
        Dex::SolfiV2 => solfi_v2::swap,
    };

    swap_function(remaining_accounts, amount_in, offset, hop_accounts, hop, proxy_from, owner_seeds)
}
