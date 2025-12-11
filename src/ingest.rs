use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ahash::HashMapExt;
use futures_util::StreamExt as _;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey, pubkey::Pubkey};
use tracing::{error, info};
use utoipa::openapi::info;
use yellowstone_grpc_client::{GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::geyser::subscribe_update;

use crate::{
    Markets, Programs, StateAccountToMarket, StateTransmitter, TransmitState,
    adapters::amms::{AccountMap, Amm, AmmContext, KeyedAccount, OBRIC_V2, RAYDIUM_CL, RAYDIUM_CP, SOLFI_V1, SOLFI_V2, obric_v2::integration::ObricV2, raydium_cp::RaydiumCP},
    bootstrap::MarketRaw,
    error,
    geyser_client::GeyserClientWrapped,
    helpers::{deserialize_anchor_account, geyser_acc_to_native},
};

/// ..
#[async_trait::async_trait]
pub trait Ingest: Send + Sync {
    fn name(&self) -> &str;

    async fn ingest(&mut self, state: StateTransmitter) -> eyre::Result<()>;
}

pub struct IngestorCfg<T: Interceptor + Send + Sync> {
    pub client_geyser: GeyserGrpcClient<T>,
    pub client_default: std::sync::Arc<RpcClient>,
    pub program_markets: Programs,
    pub markets: Markets,
    pub account_map: AccountMap,
}

pub struct GeyserPoolStateIngestor<T: Interceptor + Send + Sync> {
    client_geyser: GeyserClientWrapped<T>,
    client_default: std::sync::Arc<RpcClient>,
    program_markets: Programs,
    markets: Markets,
    account_map: AccountMap,
}

impl<T: Interceptor + Send + Sync> GeyserPoolStateIngestor<T> {
    pub fn new(cfg: IngestorCfg<T>) -> Self {
        Self {
            client_geyser: GeyserClientWrapped::new(cfg.client_geyser),
            client_default: cfg.client_default,
            program_markets: cfg.program_markets,
            markets: cfg.markets,
            account_map: cfg.account_map,
        }
    }
}

#[async_trait::async_trait]
impl<T: Interceptor + Send + Sync> Ingest for GeyserPoolStateIngestor<T> {
    fn name(&self) -> &str {
        "GeyserPoolStateIngestor"
    }

    async fn ingest(&mut self, _: StateTransmitter) -> eyre::Result<()> {
        info!("starting service: {}", self.name() /* self.markets.len() */,);

        let state_acc_to_market: StateAccountToMarket = self
            .markets
            .lock()
            .unwrap()
            .values()
            .into_iter()
            .map(|market| {
                let accs = market.get_accounts_to_update();
                accs.into_iter().map(|acc| (acc, market.key()))
            })
            .flatten()
            .collect();

        let filter = self.client_geyser.craft_filter(state_acc_to_market.keys().map(|v| v.to_string()).collect()).await;
        let mut stream = self.client_geyser.subscribe(filter).await;

        // need to init through ::from_keyed_account()

        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    if let Some(update) = msg.update_oneof
                        && let subscribe_update::UpdateOneof::Account(account_update) = update
                        && let Some(account_info) = account_update.account
                    {
                        let pubkey = Pubkey::try_from(account_info.pubkey.as_slice()).expect("Invalid pubkey");
                        let account = geyser_acc_to_native(&account_info);
                        self.account_map.insert(pubkey, account);

                        // we don't need to send a msg to `Strategy` since we're sharing the underlying structure
                        let market_pubkey = state_acc_to_market.get(&pubkey).unwrap();
                        if let Some(market) = self.markets.lock().unwrap().get_mut(market_pubkey) {
                            match market.update(&self.account_map) {
                                Ok(_) => {
                                    info!("recv update")
                                }
                                Err(_) => {}
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("received unsupported message - {}", e);
                    // metrics?
                }
            }
        }

        Ok(())
    }
}
