use anchor_lang::prelude::*;

#[constant]
pub const SEED_SA: &[u8] = b"router";
pub const BUMP_SA: u8 = 251;

pub const COMMISSION_RATE_LIMIT: u16 = 1_000; // 10%
pub const COMMISSION_DENOMINATOR: u64 = 10_000;

pub const COMMISSION_RATE_LIMIT_V2: u32 = 100_000_000; // 10%
pub const COMMISSION_DENOMINATOR_V2: u64 = 1_000_000_000;

pub const PLATFORM_FEE_RATE_LIMIT_V2: u64 = 1_000_000_000; // 100%
pub const PLATFORM_FEE_DENOMINATOR_V2: u64 = 1_000_000_000;

pub const TRIM_RATE_LIMIT_V2: u8 = 100; // 10%
pub const TRIM_DENOMINATOR_V2: u16 = 1_000;

pub const PLATFORM_FEE_RATE_LIMIT_V3: u64 = 10_000; // 100%
pub const PLATFORM_FEE_DENOMINATOR_V3: u64 = 10_000;

pub const MAX_HOPS: usize = 3;
pub const TOTAL_WEIGHT: u8 = 100;
pub const SA_AUTHORITY_SEED: &[&[&[u8]]] = &[&[SEED_SA, &[BUMP_SA]]];
pub const TOKEN_ACCOUNT_RENT: u64 = 2039280; // Token account rent (165 bytes)
pub const MIN_SOL_ACCOUNT_RENT: u64 = 890880;

// Actual amount_in lower bound ratio for post swap check
pub const ACTUAL_IN_LOWER_BOUND_NUM: u128 = 90; // 90%
pub const ACTUAL_IN_LOWER_BOUND_DEN: u128 = 100; // denominator for percentage

pub const SWAP_SELECTOR: &[u8; 8] = &[248, 198, 158, 145, 225, 117, 135, 200];
pub const SWAP2_SELECTOR: &[u8; 8] = &[65, 75, 63, 76, 235, 91, 91, 136];
pub const CPSWAP_SELECTOR: &[u8; 8] = &[143, 190, 90, 218, 196, 30, 51, 222];
pub const SWAP_V2_SELECTOR: &[u8; 8] = &[43, 4, 237, 11, 26, 201, 30, 98];
pub const ZERO_ADDRESS: Pubkey = Pubkey::new_from_array([0u8; 32]);

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

pub mod authority_pda {
    use anchor_lang::declare_id;
    declare_id!("HV1KXxWFaSeriyFvXyx48FqG9BoFbfinB8njCJonqP7K");
}

pub mod token_2022_program {
    use anchor_lang::declare_id;
    declare_id!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
}

pub mod wsol_program {
    use anchor_lang::declare_id;
    declare_id!("So11111111111111111111111111111111111111112");
}

pub mod system_program {
    use anchor_lang::declare_id;
    declare_id!("11111111111111111111111111111111");
}
