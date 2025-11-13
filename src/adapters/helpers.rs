use std::convert::TryInto;

use anchor_lang::{
    ToAccountMetas,
    prelude::{AccountMeta, Pubkey},
};
use anyhow::{Context, Result, ensure};
pub use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, AmmLabel, AmmProgramIdToLabel, KeyedAccount, KeyedUiAccount, Quote, QuoteParams, Side, SingleProgramAmm, Swap, SwapAndAccountMetas, SwapMode,
    SwapParams, single_program_amm, try_get_account_data,
};
use rust_decimal::Decimal;
use solana_program::program_pack::Pack;
use spl_token::state::Account as TokenAccount;

use crate::curves::{
    base::SwapCurve,
    calculator::TradeDirection,
    fees::{Fees, Fees as TokenSwapFees},
};

pub fn to_dex_account_metas(program_id: anchor_lang::prelude::Pubkey, accounts: impl ToAccountMetas) -> Vec<AccountMeta> {
    let mut account_metas = vec![AccountMeta::new_readonly(program_id, false)];
    account_metas.extend(accounts.to_account_metas(None));
    account_metas
}

#[derive(Copy, Clone, Debug)]
pub struct TokenSwap {
    pub token_swap_program: Pubkey,
    pub token_program: Pubkey,
    pub swap: Pubkey,
    pub authority: Pubkey,
    pub user_transfer_authority: Pubkey,
    pub source: Pubkey,
    pub swap_source: Pubkey,
    pub swap_destination: Pubkey,
    pub destination: Pubkey,
    pub pool_mint: Pubkey,
    pub pool_fee: Pubkey,
}

impl From<TokenSwap> for Vec<AccountMeta> {
    fn from(accounts: TokenSwap) -> Self {
        vec![
            AccountMeta::new_readonly(accounts.token_swap_program, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.swap, false),
            AccountMeta::new_readonly(accounts.authority, false),
            AccountMeta::new_readonly(accounts.user_transfer_authority, false),
            AccountMeta::new(accounts.source, false),
            AccountMeta::new(accounts.swap_source, false),
            AccountMeta::new(accounts.swap_destination, false),
            AccountMeta::new(accounts.destination, false),
            AccountMeta::new(accounts.pool_mint, false),
            AccountMeta::new(accounts.pool_fee, false),
        ]
    }
}

impl ToAccountMetas for TokenSwap {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
        vec![AccountMeta::default()]
    }
}

//pub trait TokenSwap {
//    fn exchange(&self, token_amounts: &[u128], in_amount: u128, input_index: usize, output_index: Option<usize>) -> Option<SwapResult>;
//}

#[derive(Debug, Clone, Default)]
pub struct SwapResult {
    pub fee_pct: Decimal,
    pub fees: u128,
    pub input_amount: u128,
    pub expected_output_amount: u128,
}

pub fn get_swap_curve_result(
    swap_curve: &SwapCurve,
    amount: u64,
    swap_source_amount: u128,
    swap_destination_amount: u128,
    trade_direction: TradeDirection,
    fees: &TokenSwapFees,
) -> Result<SwapResult> {
    let curve_result = swap_curve.swap(amount.into(), swap_source_amount, swap_destination_amount, trade_direction, fees).context("quote failed")?;

    let fees = Fees::new(fees.trade_fee_numerator, fees.trade_fee_denominator, fees.owner_trade_fee_numerator, fees.owner_trade_fee_denominator);
    let fee_pct = fees.fee_pct().context("failed to get fee pct")?;

    Ok(SwapResult {
        expected_output_amount: curve_result.destination_amount_swapped,
        fees: curve_result.trade_fee + curve_result.owner_fee,
        input_amount: curve_result.source_amount_swapped,
        fee_pct,
        ..Default::default()
    })
}
