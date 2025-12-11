//! Magnus is a modular Solana solver; There are a few things to note here:
//! TODO
//! |1| - ..
//! |2| - ..
//! |3| - ..

pub mod adapters;
pub mod bootstrap;
pub mod curves;
pub mod error;
pub mod geyser_client;
pub mod helpers;
pub mod ingest;
pub mod payload;
pub mod solve;

pub trait TransmitState: Send + Sync {}
pub trait ExecuteSignal: Send + Sync {}

#[derive(Copy, Clone, Debug)]
pub struct StateTransmitter;
impl TransmitState for StateTransmitter {}

#[derive(Copy, Clone, Debug)]
pub struct SignalExecutor;
impl ExecuteSignal for SignalExecutor {}

/// HashMap<Pubkey, Vec<Pubkey>>
///
///   -> the key is the program (amm) addr
///   -> the value is a list of the markets we collect data for
pub type Programs = std::collections::HashMap<solana_sdk::pubkey::Pubkey, Vec<solana_sdk::pubkey::Pubkey>>;

/// Arc<Mutex<HashMap<Pubkey, Box<dyn Amm>>>>
///
///   -> the pubkey is the market addr
///   -> the value is the actual market impl
pub type Markets = std::sync::Arc<std::sync::Mutex<std::collections::HashMap<solana_sdk::pubkey::Pubkey, Box<dyn crate::adapters::amms::Amm>>>>;

/// HashMap<Pubkey, Pubkey>
///
///   -> the key is an account addr we receive subscription updates for
///   -> the value is the market addr (i.e the 'owner acc' of the key account addr)
pub type StateAccountToMarket = std::collections::HashMap<solana_sdk::pubkey::Pubkey, solana_sdk::pubkey::Pubkey>;

/// HashMap<Pubkey, Account> (aka AccountMap)
///   -> the key is the account that we follow for updates
///   -> the value is the actual account structure
pub type AccountMap = std::collections::HashMap<solana_sdk::pubkey::Pubkey, solana_sdk::account::Account, ahash::RandomState>;

#[derive(Copy, Clone, Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SrcKind {
    // get the best pricing from any aggregator
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
