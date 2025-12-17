use std::{fmt::Debug, str::FromStr};

use litesvm::LiteSVM;
use magnus_consts::pmm_humidifi;
use solana_sdk::pubkey::Pubkey;

use crate::adapters::{Adapter, amms::Amm};

/*
 * Few things that might be more opaque here:
 *
 * Since we cannot directly deserialize into some structure (there's no clue how
 * humidifi, or any other prop AMM for that matter, keeps track of its state),
 * we'll simulate the `quote` and `swap` expected by the `Amm` trait through
 * a virtual env established through litesvm.
 */
#[derive(Clone, Default)]
pub struct Humidifi {
    key: Pubkey,
    involved_accounts: Vec<Pubkey>,
    svm: LiteSVM,
}

impl Adapter for Humidifi {}

impl Humidifi {
    pub fn new(key: Pubkey, involved_accounts: Vec<String>) -> Humidifi {
        // we'll need a proper way to setup the SVM such that
        // the router & humidifi programs are loaded, as well as the current acc
        // info
        // let svm = LiteSVM::new().with_default_programs();

        let involved_accounts = involved_accounts.iter().map(|v| Pubkey::from_str(v).unwrap()).collect();
        Humidifi { key, involved_accounts, ..Humidifi::default() }
    }
}

impl std::fmt::Debug for Humidifi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Humidifi | key {} | involved accounts - {:?}", self.key, self.involved_accounts))
    }
}

impl Amm for Humidifi {
    fn program_id(&self) -> Pubkey {
        Pubkey::from_str_const(&pmm_humidifi::id().to_string())
    }

    fn label(&self) -> String {
        "HumidiFi".to_string()
    }

    fn get_accounts_len(&self) -> usize {
        pmm_humidifi::ACCOUNTS_LEN
    }

    fn key(&self) -> solana_sdk::pubkey::Pubkey {
        self.key
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        // we don't store (nor know) the reserve mints
        // nevertheless we can still simulate locally
        vec![]
    }

    fn get_accounts_to_update(&self) -> Vec<solana_sdk::pubkey::Pubkey> {
        self.involved_accounts.clone()
    }

    fn update(&mut self, _account_map: &super::AccountMap, _slot: Option<u64>) -> eyre::Result<()> {
        /*
         * Since there's no way to keep state for a particular AMM
         */
        Ok(())
    }

    fn quote(&self, _quote_params: &crate::adapters::QuoteParams) -> eyre::Result<crate::adapters::Quote> {
        unimplemented!()
    }

    fn get_swap_and_account_metas(&self, _swap_params: &crate::adapters::SwapParams) -> eyre::Result<crate::adapters::SwapAndAccountMetas> {
        unimplemented!()
    }

    fn from_keyed_account(_keyed_account: &super::KeyedAccount, _amm_context: &super::AmmContext) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }
}
