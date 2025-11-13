use std::sync::Arc;

use anyhow::Context;
use rust_decimal::Decimal;

use crate::fees::TokenSwapFees;

pub trait TokenSwap {
    fn exchange(&self, token_amounts: &[u128], in_amount: u128, input_index: usize, output_index: Option<usize>) -> Option<SwapResult>;
}

//#[derive(Debug, Clone, Default)]
//pub struct SwapResult {
//    pub fee_pct: Decimal,
//    pub fees: u128,
//    pub input_amount: u128,
//    pub expected_output_amount: u128,
//}

#[derive(Copy, Clone, Debug)]
pub enum CurveKind {
    ConstantProduct,
}

pub trait CurveCalculator {}

pub struct SwapCurve {
    /// The type of curve contained in the calculator, helpful for outside
    /// queries
    pub curve_type: CurveKind,
    /// The actual calculator, represented as a trait object to allow for many
    /// different types of curves
    pub calculator: Arc<dyn CurveCalculator + Sync + Send>,
}

/// Encodes all results of swapping from a source token to a destination token
#[derive(Debug, PartialEq)]
pub struct SwapWithoutFeesResult {
    /// Amount of source token swapped
    pub source_amount_swapped: u128,
    /// Amount of destination token swapped
    pub destination_amount_swapped: u128,
}

impl SwapCurve {
    /// Subtract fees and calculate how much destination token will be provided
    /// given an amount of source token.
    pub fn swap(&self, source_amount: u128, swap_source_amount: u128, swap_destination_amount: u128, trade_direction: TradeDirection, fees: &Fees) -> Option<SwapResult> {
        // debit the fee to calculate the amount swapped
        let trade_fee = fees.trading_fee(source_amount)?;
        let owner_fee = fees.owner_trading_fee(source_amount)?;

        let total_fees = trade_fee.checked_add(owner_fee)?;
        let source_amount_less_fees = source_amount.checked_sub(total_fees)?;

        let SwapWithoutFeesResult { source_amount_swapped, destination_amount_swapped } =
            self.calculator.swap_without_fees(source_amount_less_fees, swap_source_amount, swap_destination_amount, trade_direction)?;

        let source_amount_swapped = source_amount_swapped.checked_add(total_fees)?;
        Some(SwapResult {
            new_swap_source_amount: swap_source_amount.checked_add(source_amount_swapped)?,
            new_swap_destination_amount: swap_destination_amount.checked_sub(destination_amount_swapped)?,
            source_amount_swapped,
            destination_amount_swapped,
            trade_fee,
            owner_fee,
        })
    }

    /// Get the amount of pool tokens for the deposited amount of token A or B
    pub fn deposit_single_token_type(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
        fees: &Fees,
    ) -> Option<u128> {
        if source_amount == 0 {
            return Some(0);
        }
        // Get the trading fee incurred if *half* the source amount is swapped
        // for the other side. Reference at:
        // https://github.com/balancer-labs/balancer-core/blob/f4ed5d65362a8d6cec21662fb6eae233b0babc1f/contracts/BMath.sol#L117
        let half_source_amount = std::cmp::max(1, source_amount.checked_div(2)?);
        let trade_fee = fees.trading_fee(half_source_amount)?;
        let owner_fee = fees.owner_trading_fee(half_source_amount)?;
        let total_fees = trade_fee.checked_add(owner_fee)?;
        let source_amount = source_amount.checked_sub(total_fees)?;
        self.calculator.deposit_single_token_type(source_amount, swap_token_a_amount, swap_token_b_amount, pool_supply, trade_direction)
    }

    /// Get the amount of pool tokens for the withdrawn amount of token A or B
    pub fn withdraw_single_token_type_exact_out(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
        fees: &Fees,
    ) -> Option<u128> {
        if source_amount == 0 {
            return Some(0);
        }
        // Since we want to get the amount required to get the exact amount out,
        // we need the inverse trading fee incurred if *half* the source amount
        // is swapped for the other side. Reference at:
        // https://github.com/balancer-labs/balancer-core/blob/f4ed5d65362a8d6cec21662fb6eae233b0babc1f/contracts/BMath.sol#L117
        let half_source_amount = source_amount.checked_add(1)?.checked_div(2)?; // round up
        let pre_fee_source_amount = fees.pre_trading_fee_amount(half_source_amount)?;
        let source_amount = source_amount.checked_sub(half_source_amount)?.checked_add(pre_fee_source_amount)?;
        self.calculator.withdraw_single_token_type_exact_out(source_amount, swap_token_a_amount, swap_token_b_amount, pool_supply, trade_direction, RoundDirection::Ceiling)
    }
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

#[derive(Debug, Clone, Default)]
pub struct SwapResult {
    pub fee_pct: Decimal,
    pub fees: u128,
    pub input_amount: u128,
    pub expected_output_amount: u128,
}
