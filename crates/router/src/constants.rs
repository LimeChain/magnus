use anchor_lang::prelude::*;

#[constant]
pub const SEED_SA: &[u8] = b"magnus-router";
pub const BUMP_SA: u8 = 251;

pub const MAX_HOPS: usize = 3;
pub const TOTAL_WEIGHT: u8 = 100;
pub const SA_AUTHORITY_SEED: &[&[&[u8]]] = &[&[SEED_SA, &[BUMP_SA]]];

// Actual amount_in lower bound ratio for post swap check
pub const ACTUAL_IN_LOWER_BOUND_NUM: u128 = 90; // 90%
pub const ACTUAL_IN_LOWER_BOUND_DEN: u128 = 100; // denominator for percentage

pub const ZERO_ADDRESS: Pubkey = Pubkey::new_from_array([0u8; 32]);
