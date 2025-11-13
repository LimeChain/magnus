use anyhow::Context;
use raydium_cp_swap::curve::{Fees, TradeDirection};
use rust_decimal::Decimal;

use crate::fees::TokenSwapFees;

pub trait TokenSwap {
    fn exchange(&self, token_amounts: &[u128], in_amount: u128, input_index: usize, output_index: Option<usize>) -> Option<SwapResult>;
}

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
) -> eyre::Result<SwapResult> {
    let curve_result = swap_curve.swap(amount.into(), swap_source_amount, swap_destination_amount, trade_direction, fees).context("quote failed")?;

    let fees = TokenSwapFees::new(fees.trade_fee_numerator, fees.trade_fee_denominator, fees.owner_trade_fee_numerator, fees.owner_trade_fee_denominator);
    let fee_pct = fees.fee_pct().context("failed to get fee pct")?;

    Ok(SwapResult {
        expected_output_amount: curve_result.destination_amount_swapped,
        fees: curve_result.trade_fee + curve_result.owner_fee,
        input_amount: curve_result.source_amount_swapped,
        fee_pct,
        ..Default::default()
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveType {
    /// Uniswap-style constant product curve, invariant = token_a_amount *
    /// token_b_amount
    ConstantProduct,
}
