use crate::adapters::{
    Adapter,
    amms::{Amm, HUMIDIFI},
};

/*
 * Few things that might be more opaque here:
 *
 * Since we cannot directly deserialize into some structure (there's no clue how
 * humidifi, or any other prop AMM for that matter, keeps track of its state),
 * we'll simulate the `quote` and `swap` expected by the `Amm` trait through
 * a virtual env established through litesvm.
 */
pub struct Humidifi;

impl Adapter for Humidifi {}

impl Amm for Humidifi {
    fn program_id(&self) -> solana_sdk::pubkey::Pubkey {
        HUMIDIFI
    }

    fn label(&self) -> String {
        "HumidiFi".to_string()
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
