use std::{fmt::Debug, path::Path, time::SystemTime};

use litesvm::{LiteSVM, types::TransactionMetadata};
use serde::{Deserialize, Serialize};
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_sdk::{account::Account, program_pack::Pack, pubkey::Pubkey, rent::Rent, signature::Keypair, signer::Signer};
use spl_associated_token_account::get_associated_token_address;

use crate::{
    AccountMap,
    adapters::{Adapter, Quote, QuoteParams, SwapAndAccountMetas, SwapParams},
};

pub mod humidifi;
pub mod obric_v2;
pub mod raydium_cp;
pub mod swap_state;

/// ..
pub trait Amm: Adapter + Send + Sync + Debug {
    fn from_keyed_account(keyed_account: &KeyedAccount) -> eyre::Result<Self>
    where
        Self: Sized;

    fn label(&self) -> String;
    fn program_id(&self) -> Pubkey;
    fn key(&self) -> Pubkey;
    fn get_reserve_mints(&self) -> Vec<Pubkey>;
    fn get_accounts_to_update(&self) -> Vec<Pubkey>;
    fn update(&mut self, account_map: &AccountMap, slot: Option<u64>) -> eyre::Result<()>;
    fn quote(&mut self, quote_params: &QuoteParams) -> eyre::Result<Quote>;
    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> eyre::Result<SwapAndAccountMetas>;

    /// Indicates if get_accounts_to_update might return a non constant vec
    fn has_dynamic_accounts(&self) -> bool {
        false
    }

    /// Indicates whether `update` needs to be called before `get_reserve_mints`
    fn requires_update_for_reserve_mints(&self) -> bool {
        false
    }

    // Indicates that whether ExactOut mode is supported
    fn supports_exact_out(&self) -> bool {
        false
    }

    /// It can only trade in one direction from its first mint to second mint, assuming it is a two mint AMM
    fn unidirectional(&self) -> bool {
        false
    }

    fn get_accounts_len(&self) -> usize {
        32
    }

    fn is_active(&self) -> bool {
        true
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyedAccount {
    pub key: Pubkey,
    pub account: Account,
    pub params: Option<serde_json::Value>,
}

#[derive(Copy, Clone, Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    // get the best pricing from all aggregators
    Aggregators,

    // poke a particular aggregator for quote/swap
    Jupiter,
    DFlow,

    // get the best pricing from any of the integrated AMMs
    // perhaps we can get even more granular here and segment into (prop|public) AMMs
    #[serde(rename = "amms")]
    #[default]
    AMMs,
}

pub struct Chroot {
    pub svm: LiteSVM,
    pub wallet: Keypair,
    pub mints: [(Pubkey, u8); 2],
}

impl Chroot {
    const AIRDROP_AMOUNT: u64 = 100_000_000;
    const BUDGET_AMOUNT: u64 = 20_000_000;

    pub fn new(mints: [(Pubkey, u8); 2]) -> Self {
        let mut budget = ComputeBudget::new_with_defaults(true);
        budget.compute_unit_limit = Self::BUDGET_AMOUNT;

        let svm = LiteSVM::new().with_default_programs().with_sysvars().with_sigverify(true).with_compute_budget(budget);
        let wallet = Keypair::new();

        let mut chroot = Chroot { svm, wallet, mints };

        chroot.svm.airdrop(&&chroot.wallet_pubkey(), Chroot::AIRDROP_AMOUNT).unwrap();

        mints.iter().for_each(|(mint, dec)| {
            let mint_acc = Chroot::mk_mint_acc(*dec);
            chroot.load_accounts(vec![(*mint, mint_acc)]).expect(&format!("unable to load account {}", *mint));
        });

        chroot
    }

    pub fn load_program(&mut self, pubkey: Pubkey, program: impl AsRef<Path>) -> eyre::Result<()> {
        self.svm.add_program_from_file(pubkey, &program)?;

        Ok(())
    }

    pub fn load_accounts(&mut self, accs: Vec<(Pubkey, Account)>) -> eyre::Result<()> {
        accs.iter().try_for_each(|(pubkey, account)| self.svm.set_account(*pubkey, account.clone()))?;

        Ok(())
    }

    pub fn update_accounts(&mut self, accs: Vec<(Pubkey, Account)>) -> eyre::Result<()> {
        self.load_accounts(accs)?;

        Ok(())
    }

    pub fn update_slot(&mut self, slot: u64) {
        self.svm.warp_to_slot(slot);
    }

    /// Creates fully initialised mint account suitable for use in LiteSVM simulations.
    fn mk_mint_acc(decimals: u8) -> Account {
        let mint = spl_token::state::Mint {
            mint_authority: solana_sdk::program_option::COption::None,
            supply: u64::MAX,
            decimals,
            is_initialized: true,
            freeze_authority: Default::default(),
        };

        let mut data = vec![0u8; spl_token::state::Mint::LEN];
        spl_token::state::Mint::pack(mint, &mut data).unwrap();

        Account { lamports: Rent::default().minimum_balance(data.len()), data, owner: spl_token::id(), executable: false, rent_epoch: u64::MAX }
    }

    /// Creates a mock SPL Token Account (ATA) with the specified balance.
    fn mk_ata(mint: &Pubkey, user: &Pubkey, amount: u64) -> Account {
        let ata = spl_token::state::Account { mint: *mint, owner: *user, amount, state: spl_token::state::AccountState::Initialized, ..Default::default() };

        let mut data = vec![0u8; spl_token::state::Account::LEN];
        ata.pack_into_slice(&mut data);

        Account { lamports: Rent::default().minimum_balance(data.len()), data, owner: spl_token::id(), executable: false, rent_epoch: u64::MAX }
    }

    pub fn wallet_pubkey(&self) -> Pubkey {
        self.wallet.pubkey()
    }

    pub fn wallet_ata(&self, mint: &Pubkey) -> Pubkey {
        get_associated_token_address(&mint, &self.wallet_pubkey())
    }

    pub fn get_ta(mint: Pubkey, owner: Pubkey) -> Pubkey {
        get_associated_token_address(&mint, &owner)
    }

    pub fn gen_order_id() -> u64 {
        SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
    }

    fn token_balance(&self, mint: &Pubkey) -> u64 {
        let ata = self.wallet_ata(mint);
        let acc = self.svm.get_account(&ata).unwrap_or_default();
        spl_token::state::Account::unpack(&acc.data).map(|a| a.amount).unwrap_or(0)
    }

    fn get_event_amount_out(&self, metadata: &TransactionMetadata) -> u64 {
        let amount_out: u64 = metadata
            .logs
            .iter()
            .find_map(|log| {
                if log.contains("SwapEvent") {
                    // i.e.: "Program log: SwapEvent { dex: Humidifi, amount_in: 1000000000, amount_out: 121518066 }"
                    log.split("amount_out: ").nth(1)?.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
                } else {
                    None
                }
            })
            .expect("couldn't find amount_out in logs");

        amount_out
    }
}
