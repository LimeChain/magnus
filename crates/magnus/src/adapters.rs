use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::{instruction::AccountMeta, pubkey::Pubkey, signature::Signature, transaction::Transaction};
use utoipa::ToSchema;

use crate::adapters::amms::{Side, Target};

pub mod aggregators;
pub mod amms;
pub mod helpers;

/// Defines the base traits for downstream liquidity adapters
/// Implementations usually rely on a child interface, like [`Amm`] and [`Aggregator`]
pub trait Adapter {}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Default, Debug)]
pub enum SwapMode {
    #[default]
    ExactIn,
    ExactOut,
}

#[derive(Copy, Clone, Debug)]
pub struct QuoteParams {
    pub swap_mode: SwapMode,
    pub amount: u64,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct SwapParams {
    pub swap_mode: SwapMode,
    pub amount: u64,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub source_token_account: Pubkey,
    pub destination_token_account: Pubkey,
    /// This can be the user or the program authority over the source_token_account.
    pub token_transfer_authority: Pubkey,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Quote {
    pub in_amount: u64,
    pub out_amount: u64,
    pub fee_amount: u64,
    pub fee_mint: Pubkey,
    pub fee_pct: Decimal,
}

#[derive(Clone, Debug, Default)]
pub struct SwapAndAccountMetas {
    pub swap: AmmSwap,
    pub account_metas: Vec<AccountMeta>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum AmmSwap {
    #[default]
    RaydiumCP,
    RaydiumCLV2,
    ObricV2,
    Humidifi,
}

#[derive(Clone, Debug, Default, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntQuoteResponse {
    pub source: Target,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
    pub route_plan: Option<Vec<PlanItem>>,
}

// todo: implement the ToSchema
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntSwapResponse {
    pub source: Target,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub signature: String, //Signature,
    pub route_plan: Option<Vec<PlanItem>>,
}

#[derive(Clone, Debug, Default, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub venue: String,
    pub market_key: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
}
