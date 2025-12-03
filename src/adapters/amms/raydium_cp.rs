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

    fn key(&self) -> solana_sdk::pubkey::Pubkey {
        unimplemented!()
    }

    fn get_reserve_mints(&self) -> Vec<solana_sdk::pubkey::Pubkey> {
        unimplemented!()
    }

    fn get_accounts_to_update(&self) -> Vec<solana_sdk::pubkey::Pubkey> {
        unimplemented!()
    }

    fn update(&mut self, account_map: &super::AccountMap) -> eyre::Result<()> {
        unimplemented!()
    }

    fn quote(&self, quote_params: &crate::adapters::QuoteParams) -> eyre::Result<crate::adapters::Quote> {
        unimplemented!()
    }

    fn get_swap_and_account_metas(&self, swap_params: &crate::adapters::SwapParams) -> eyre::Result<crate::adapters::SwapAndAccountMetas> {
        unimplemented!()
    }

    fn from_keyed_account(keyed_account: &super::KeyedAccount, amm_context: &super::AmmContext) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn clone_amm(&self) -> Box<dyn Amm> {
        unimplemented!()
    }
}
