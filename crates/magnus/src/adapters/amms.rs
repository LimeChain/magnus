use std::{
    boxed::Box,
    collections::HashSet,
    fmt::Debug,
    sync::{
        Arc,
        atomic::{AtomicI64, AtomicU64},
    },
};

use serde::{Deserialize, Serialize};
use solana_sdk::{account::Account, clock::Clock, pubkey::Pubkey};

use crate::{
    AccountMap,
    adapters::{Adapter, Quote, QuoteParams, SwapAndAccountMetas, SwapParams},
};

pub mod base_cl;
pub mod base_cp;
pub mod humidifi;
pub mod obric_v2;
pub mod raydium_cl;
pub mod raydium_cp;
pub mod swap_state;

pub use base_cl::BaseConcentratedLiquidityAmm;
pub use base_cp::BaseConstantProductAmm;

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

#[derive(Copy, Clone, Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LiquiditySource {
    // get the best pricing from all aggregators
    #[default]
    Aggregators,

    // poke a particular aggregator for quote/swap
    Jupiter,
    DFlow,

    // get the best pricing from any of the integrated AMMs
    // perhaps we can get even more granular here and segment into (prop|public) AMMs
    #[serde(rename = "amms")]
    AMMs,
}
