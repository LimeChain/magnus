use std::collections::HashMap;

use futures_util::StreamExt as _;
use solana_sdk::{pubkey, pubkey::Pubkey};
use tracing::{error, info};
use utoipa::openapi::info;
use yellowstone_grpc_client::{GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::geyser::subscribe_update;

use crate::{
    StateTransmitter, TransmitState,
    adapters::amms::{OBRIC_V2, RAYDIUM_CL, RAYDIUM_CP, SOLFI_V1, SOLFI_V2},
    bootstrap::Market,
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

pub struct GeyserPoolStateIngestor<T: Interceptor + Send + Sync> {
    client_geyser: GeyserClientWrapped<T>,
    markets: HashMap<Pubkey, Market>,
}

impl<T: Interceptor + Send + Sync> GeyserPoolStateIngestor<T> {
    pub fn new(client_geyser: GeyserGrpcClient<T>, markets: HashMap<Pubkey, Market>) -> Self {
        Self { client_geyser: GeyserClientWrapped::new(client_geyser), markets }
    }
}

#[async_trait::async_trait]
impl<T: Interceptor + Send + Sync> Ingest for GeyserPoolStateIngestor<T> {
    fn name(&self) -> &str {
        "GeyserPoolStateIngestor"
    }

    async fn ingest(&mut self, state: StateTransmitter) -> eyre::Result<()> {
        info!("starting service: {} | markets: {}", self.name(), self.markets.len());

        let markets = self.markets.iter().map(|(key, _)| key.to_string()).collect::<Vec<_>>();
        let filter = self.client_geyser.craft_filter(markets.clone()).await;
        let mut stream = self.client_geyser.subscribe(filter).await;

        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    if let Some(update) = msg.update_oneof
                        && let subscribe_update::UpdateOneof::Account(account_update) = update
                        && let Some(account_info) = account_update.account
                    {
                        let pubkey = Pubkey::try_from(account_info.pubkey.as_slice()).expect("Invalid pubkey");
                        let account = geyser_acc_to_native(&account_info);

                        // we'll have to match the account against a particular amm before proceeding with a concrete
                        // deserialisation format
                        // then send a meaningful msg downstream towards a `impl Strategy`
                        //
                        // 1. deserialise in the proper AMM representation
                        // 2. all AMMs should be implementing the `AMM` trait
                        // 3. the message we'll pass down will be some kind of Vec<Box<dyn Amm>>
                        // 4. since all markets will have `Amm` implemented - we'll have a clear path to
                        //    get quotes & execute swaps — both will be done in the Solve thread.
                        match account.owner {
                            SOLFI_V1 => {
                                info!("SOLFI_V1 | {:?} | {:?}", pubkey, account);
                            }
                            SOLFI_V2 => {
                                info!("SOLFI_V2 | {:?} | {:?}", pubkey, account);
                            }
                            OBRIC_V2 => {
                                info!("OBRIC_V2 | {:?} | {:?}", pubkey, account);
                            }
                            RAYDIUM_CP => {
                                info!("RAYDIUM_CP | {:?} | {:?}", pubkey, account);
                            }
                            RAYDIUM_CL => {
                                info!("RAYDIUM_CL | {:?} | {:?}", pubkey, account);
                            }
                            _ => {
                                unimplemented!("Market not yet supported - {}", pubkey);
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
