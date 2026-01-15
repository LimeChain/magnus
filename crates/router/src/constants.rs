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

pub const SWAP_SELECTOR: &[u8; 8] = &[248, 198, 158, 145, 225, 117, 135, 200];
pub const SWAP2_SELECTOR: &[u8; 8] = &[65, 75, 63, 76, 235, 91, 91, 136];
pub const SWAPV2_SELECTOR: &[u8; 8] = &[43, 4, 237, 11, 26, 201, 30, 98];
pub const CPSWAP_SELECTOR: &[u8; 8] = &[143, 190, 90, 218, 196, 30, 51, 222];
pub const TESSERA_SWAP_SELECTOR: &[u8; 1] = &[16];
pub const GOONFI_SWAP_SELECTOR: &[u8; 1] = &[2];
pub const BISONFI_SWAP_SELECTOR: u8 = 0x2;

pub const HUMIDIFI_SWAP_SELECTOR: u8 = 0x4;
const HUMIDIFI_IX_DATA_KEY_SEED: [u8; 32] =
    [58, 255, 47, 255, 226, 186, 235, 195, 123, 131, 245, 8, 11, 233, 132, 219, 225, 40, 79, 119, 169, 121, 169, 58, 197, 1, 122, 9, 216, 164, 149, 97];
pub const HUMIDIFI_IX_DATA_KEY: u64 = u64::from_le_bytes([
    HUMIDIFI_IX_DATA_KEY_SEED[0],
    HUMIDIFI_IX_DATA_KEY_SEED[1],
    HUMIDIFI_IX_DATA_KEY_SEED[2],
    HUMIDIFI_IX_DATA_KEY_SEED[3],
    HUMIDIFI_IX_DATA_KEY_SEED[4],
    HUMIDIFI_IX_DATA_KEY_SEED[5],
    HUMIDIFI_IX_DATA_KEY_SEED[6],
    HUMIDIFI_IX_DATA_KEY_SEED[7],
]);
