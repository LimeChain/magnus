use solana_sdk::pubkey::Pubkey;

use crate::adapters::{
    Adapter,
    amms::{Amm, BaseConstantProductAmm, RAYDIUM_CP},
};

#[derive(Clone, Debug)]
pub struct RaydiumCP(BaseConstantProductAmm);

impl Adapter for RaydiumCP {}

impl Amm for RaydiumCP {
    fn program_id(&self) -> Pubkey {
        RAYDIUM_CP
    }

    fn label(&self) -> String {
        "RaydiumConstantProduct".to_string()
    }

    fn get_accounts_len(&self) -> usize {
        11
    }
}
