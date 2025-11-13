use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};
use utoipa::ToSchema;

use crate::adapters::aggregators::AggregatorKind;

pub mod aggregators;
pub mod amms;

/// Defines the base traits for downstream liquidity adapters
/// Implementations usually rely on a child interface, like [`Amm`] and [`Aggregator`]
pub trait Adapter {}

#[derive(Copy, Clone, Debug)]
pub struct QuoteParams {
    pub amount: u64,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub swap_mode: SwapMode,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Default, Debug)]
pub enum SwapMode {
    #[default]
    ExactIn,
    ExactOut,
}

pub struct SwapParams {
    pub swap_mode: SwapMode,
    pub in_amount: u64,
    pub out_amount: u64,
    pub source_mint: Pubkey,
    pub destination_mint: Pubkey,
    pub source_token_account: Pubkey,
    pub destination_token_account: Pubkey,
    /// This can be the user or the program authority over the source_token_account.
    pub token_transfer_authority: Pubkey,
    /// Instead of returning the relevant Err, replace dynamic accounts with the default Pubkey
    /// This is useful for crawling market with no tick array
    pub missing_dynamic_accounts_as_default: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Quote {
    pub in_amount: u64,
    pub out_amount: u64,
    pub fee_amount: u64,
    pub fee_mint: Pubkey,
    pub fee_pct: Decimal,
}

#[derive(Clone, Debug)]
pub struct SwapAndAccountMetas {
    pub swap: Swap,
    pub account_metas: Vec<AccountMeta>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Swap {
    RaydiumCP,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub aggregator: AggregatorKind,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
    pub route_plan: Option<Vec<QuotePlanItem>>,
}

// -- internal
#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotePlanItem {
    pub venue: String,
    pub market_key: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
}
