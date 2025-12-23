use anchor_lang::prelude::*;

#[constant]
pub const SEED_SA: &[u8] = b"router";
pub const BUMP_SA: u8 = 251;

pub const MAX_HOPS: usize = 3;
pub const TOTAL_WEIGHT: u8 = 100;
pub const SA_AUTHORITY_SEED: &[&[&[u8]]] = &[&[SEED_SA, &[BUMP_SA]]];

// Actual amount_in lower bound ratio for post swap check
pub const ACTUAL_IN_LOWER_BOUND_NUM: u128 = 90; // 90%
pub const ACTUAL_IN_LOWER_BOUND_DEN: u128 = 100; // denominator for percentage

pub const SWAP_SELECTOR: &[u8; 8] = &[248, 198, 158, 145, 225, 117, 135, 200];
pub const SWAP2_SELECTOR: &[u8; 8] = &[65, 75, 63, 76, 235, 91, 91, 136];
pub const CPSWAP_SELECTOR: &[u8; 8] = &[143, 190, 90, 218, 196, 30, 51, 222];
pub const SWAP_V2_SELECTOR: &[u8; 8] = &[43, 4, 237, 11, 26, 201, 30, 98];
pub const ZERO_ADDRESS: Pubkey = Pubkey::new_from_array([0u8; 32]);

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
