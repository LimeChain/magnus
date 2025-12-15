//! The Uniswap invariant calculator.

use anchor_lang::solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use spl_math::{checked_ceil_div::CheckedCeilDiv, precise_number::PreciseNumber};

use crate::{
    curves::calculator::{CurveCalculator, DynPack, RoundDirection, SwapWithoutFeesResult, TradeDirection, map_zero_to_none},
    error::SwapError,
};

/// ConstantProductCurve struct implementing CurveCalculator
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConstantProductCurve;

/// The constant product swap calculation, factored out of its class for reuse.
///
/// This is guaranteed to work for all values such that:
///  - 1 <= swap_source_amount * swap_destination_amount <= u128::MAX
///  - 1 <= source_amount <= u64::MAX
pub fn swap(source_amount: u128, swap_source_amount: u128, swap_destination_amount: u128) -> Option<SwapWithoutFeesResult> {
    let invariant = swap_source_amount.checked_mul(swap_destination_amount)?;

    let new_swap_source_amount = swap_source_amount.checked_add(source_amount)?;
    let (new_swap_destination_amount, new_swap_source_amount) = invariant.checked_ceil_div(new_swap_source_amount)?;

    let source_amount_swapped = new_swap_source_amount.checked_sub(swap_source_amount)?;
    let destination_amount_swapped = map_zero_to_none(swap_destination_amount.checked_sub(new_swap_destination_amount)?)?;

    Some(SwapWithoutFeesResult { source_amount_swapped, destination_amount_swapped })
}

impl ConstantProductCurve {}

/// Get the amount of pool tokens for the withdrawn amount of token A or B.
///
/// The constant product implementation uses the Balancer formulas found at
/// <https://balancer.finance/whitepaper/#single-asset-withdrawal>, specifically
/// in the case for 2 tokens, each weighted at 1/2.
pub fn withdraw_single_token_type_exact_out(
    source_amount: u128,
    swap_token_a_amount: u128,
    swap_token_b_amount: u128,
    pool_supply: u128,
    trade_direction: TradeDirection,
    round_direction: RoundDirection,
) -> Option<u128> {
    let swap_source_amount = match trade_direction {
        TradeDirection::AtoB => swap_token_a_amount,
        TradeDirection::BtoA => swap_token_b_amount,
    };
    let swap_source_amount = PreciseNumber::new(swap_source_amount)?;
    let source_amount = PreciseNumber::new(source_amount)?;
    let ratio = source_amount.checked_div(&swap_source_amount)?;
    let one = PreciseNumber::new(1)?;
    let base = one.checked_sub(&ratio).unwrap_or_else(|| PreciseNumber::new(0).unwrap());
    let root = one.checked_sub(&base.sqrt()?)?;
    let pool_supply = PreciseNumber::new(pool_supply)?;
    let pool_tokens = pool_supply.checked_mul(&root)?;
    match round_direction {
        RoundDirection::Floor => pool_tokens.floor()?.to_imprecise(),
        RoundDirection::Ceiling => pool_tokens.ceiling()?.to_imprecise(),
    }
}

/// Calculates the total normalized value of the curve given the liquidity
/// parameters.
///
/// The constant product implementation for this function gives the square root
/// of the Uniswap invariant.
pub fn normalized_value(swap_token_a_amount: u128, swap_token_b_amount: u128) -> Option<PreciseNumber> {
    let swap_token_a_amount = PreciseNumber::new(swap_token_a_amount)?;
    let swap_token_b_amount = PreciseNumber::new(swap_token_b_amount)?;
    swap_token_a_amount.checked_mul(&swap_token_b_amount)?.sqrt()
}

impl CurveCalculator for ConstantProductCurve {
    fn swap_without_fees(&self, source_amount: u128, swap_source_amount: u128, swap_destination_amount: u128, _trade_direction: TradeDirection) -> Option<SwapWithoutFeesResult> {
        swap(source_amount, swap_source_amount, swap_destination_amount)
    }

    fn withdraw_single_token_type_exact_out(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
        round_direction: RoundDirection,
    ) -> Option<u128> {
        withdraw_single_token_type_exact_out(source_amount, swap_token_a_amount, swap_token_b_amount, pool_supply, trade_direction, round_direction)
    }

    fn normalized_value(&self, swap_token_a_amount: u128, swap_token_b_amount: u128) -> Option<PreciseNumber> {
        normalized_value(swap_token_a_amount, swap_token_b_amount)
    }

    fn validate(&self) -> Result<(), SwapError> {
        Ok(())
    }
}

/// IsInitialized is required to use `Pack::pack` and `Pack::unpack`
impl IsInitialized for ConstantProductCurve {
    fn is_initialized(&self) -> bool {
        true
    }
}
impl Sealed for ConstantProductCurve {}
impl Pack for ConstantProductCurve {
    const LEN: usize = 0;

    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }

    fn unpack_from_slice(_input: &[u8]) -> Result<ConstantProductCurve, ProgramError> {
        Ok(Self {})
    }
}

impl DynPack for ConstantProductCurve {
    fn pack_into_slice(&self, _output: &mut [u8]) {}
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::curves::calculator::test::check_curve_value_from_swap;

    #[test]
    fn pack_constant_product_curve() {
        let curve = ConstantProductCurve {};

        let mut packed = [0u8; ConstantProductCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = ConstantProductCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let packed = vec![];
        let unpacked = ConstantProductCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }

    fn test_truncation(
        curve: &ConstantProductCurve,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        expected_source_amount_swapped: u128,
        expected_destination_amount_swapped: u128,
    ) {
        let invariant = swap_source_amount * swap_destination_amount;
        let result = curve.swap_without_fees(source_amount, swap_source_amount, swap_destination_amount, TradeDirection::AtoB).unwrap();
        assert_eq!(result.source_amount_swapped, expected_source_amount_swapped);
        assert_eq!(result.destination_amount_swapped, expected_destination_amount_swapped);
        let new_invariant = (swap_source_amount + result.source_amount_swapped) * (swap_destination_amount - result.destination_amount_swapped);
        assert!(new_invariant >= invariant);
    }

    #[test]
    fn constant_product_swap_rounding() {
        let curve = ConstantProductCurve;

        // much too small
        assert!(curve.swap_without_fees(10, 70_000_000_000, 4_000_000, TradeDirection::AtoB).is_none()); // spot: 10 * 4m / 70b = 0

        let tests: &[(u128, u128, u128, u128, u128)] = &[
            // spot: 10 * 70b / ~4m = 174,999.99
            (10, 4_000_000, 70_000_000_000, 10, 174_999),
            // spot: 20 * 1 / 3.000 = 6.6667 (source can be 18 to get 6 dest.)
            (20, 30_000 - 20, 10_000, 18, 6),
            // spot: 19 * 1 / 2.999 = 6.3334 (source can be 18 to get 6 dest.)
            (19, 30_000 - 20, 10_000, 18, 6),
            // spot: 18 * 1 / 2.999 = 6.0001
            (18, 30_000 - 20, 10_000, 18, 6),
            // spot: 10 * 3 / 2.0010 = 14.99
            (10, 20_000, 30_000, 10, 14),
            // spot: 10 * 3 / 2.0001 = 14.999
            (10, 20_000 - 9, 30_000, 10, 14),
            // spot: 10 * 3 / 2.0000 = 15
            (10, 20_000 - 10, 30_000, 10, 15),
            // spot: 100 * 3 / 6.001 = 49.99 (source can be 99 to get 49 dest.)
            (100, 60_000, 30_000, 99, 49),
            // spot: 99 * 3 / 6.001 = 49.49
            (99, 60_000, 30_000, 99, 49),
            // spot: 98 * 3 / 6.001 = 48.99 (source can be 97 to get 48 dest.)
            (98, 60_000, 30_000, 97, 48),
        ];
        for (source_amount, swap_source_amount, swap_destination_amount, expected_source_amount, expected_destination_amount) in tests.iter() {
            test_truncation(&curve, *source_amount, *swap_source_amount, *swap_destination_amount, *expected_source_amount, *expected_destination_amount);
        }
    }

    proptest! {
        #[test]
        fn curve_value_does_not_decrease_from_swap(
            source_token_amount in 1..u64::MAX,
            swap_source_amount in 1..u64::MAX,
            swap_destination_amount in 1..u64::MAX,
        ) {
            let curve = ConstantProductCurve {};
            check_curve_value_from_swap(
                &curve,
                source_token_amount as u128,
                swap_source_amount as u128,
                swap_destination_amount as u128,
                TradeDirection::AtoB
            );
        }
    }
}
