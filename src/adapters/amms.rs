use std::{
    boxed::Box,
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::{
        Arc,
        atomic::{AtomicI64, AtomicU64},
    },
};

use serde::{Deserialize, Serialize};
use solana_sdk::{account::Account, clock::Clock, pubkey, pubkey::Pubkey};

use crate::adapters::{Adapter, Quote, QuoteParams, SwapAndAccountMetas, SwapParams};

pub mod base_cl;
pub mod base_cp;
pub mod humidifi;
pub mod obric_v2;
//pub mod openbook_v2;
pub mod raydium_cl;
pub mod raydium_cp;
pub mod swap_state;

pub use base_cl::BaseConcentratedLiquidityAmm;
pub use base_cp::BaseConstantProductAmm;

pub const SPL_TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

pub const SOLFI_V1: Pubkey = pubkey!("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe");
pub const SOLFI_V2: Pubkey = pubkey!("SV2EYYJyRz2YhfXwXnhNAevDEui5Q6yrfyo13WtupPF");
pub const RAYDIUM_CP: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
pub const RAYDIUM_CL: Pubkey = pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
pub const HUMIDIFI: Pubkey = pubkey!("9H6tua7jkLhdm3w8BvgpTn5LZNU7g4ZynDmCiNN3q6Rp");
pub const OBRIC_V2: Pubkey = pubkey!("obriQD1zbpyLz95G5n7nJe6a4DPjpFwa5XYPoNm113y");
pub const OPENBOOK_V2: Pubkey = pubkey!("opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb");

// HashMap<Pubkey, Account> (aka AccountMap)
//   -> the key is the account that we follow for updates
//   -> the value is the account structure
pub type AccountMap = HashMap<Pubkey, Account, ahash::RandomState>;

/// ..
pub trait Amm: Adapter + Send + Sync + Debug {
    fn from_keyed_account(keyed_account: &KeyedAccount, amm_context: &AmmContext) -> eyre::Result<Self>
    where
        Self: Sized;

    fn label(&self) -> String;
    fn program_id(&self) -> Pubkey;
    fn key(&self) -> Pubkey;
    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync>;
    fn get_reserve_mints(&self) -> Vec<Pubkey>;
    fn get_accounts_to_update(&self) -> Vec<Pubkey>;
    fn update(&mut self, account_map: &AccountMap) -> eyre::Result<()>;
    fn quote(&self, quote_params: &QuoteParams) -> eyre::Result<Quote>;
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
        32 // Default to a large num to penalise no impl
    }

    /// The identifier of the underlying liquidity
    ///
    /// Example:
    /// For RaydiumAmm uses Openbook market A this will return Some(A)
    /// For Openbook market A, it will also return Some(A)
    fn underlying_liquidities(&self) -> Option<HashSet<Pubkey>> {
        None
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

#[derive(Clone, Debug, Default)]
pub struct ClockRef {
    pub slot: Arc<AtomicU64>,
    /// The timestamp of the first `Slot` in this `Epoch`.
    pub epoch_start_timestamp: Arc<AtomicI64>,
    /// The current `Epoch`.
    pub epoch: Arc<AtomicU64>,
    pub leader_schedule_epoch: Arc<AtomicU64>,
    pub unix_timestamp: Arc<AtomicI64>,
}

impl ClockRef {
    pub fn update(&self, clock: Clock) {
        self.epoch.store(clock.epoch, std::sync::atomic::Ordering::Relaxed);
        self.slot.store(clock.slot, std::sync::atomic::Ordering::Relaxed);
        self.unix_timestamp.store(clock.unix_timestamp, std::sync::atomic::Ordering::Relaxed);
        self.epoch_start_timestamp.store(clock.epoch_start_timestamp, std::sync::atomic::Ordering::Relaxed);
        self.leader_schedule_epoch.store(clock.leader_schedule_epoch, std::sync::atomic::Ordering::Relaxed);
    }
}

#[derive(Clone, Debug, Default)]
pub struct AmmContext {
    pub clock_ref: ClockRef,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Side {
    Bid,
    Ask,
}
