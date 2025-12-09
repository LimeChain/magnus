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
    Markets, Programs, StateTransmitter, TransmitState,
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

    async fn ingest(&mut self, state: StateTransmitter) -> eyre::Result<()> {
        info!("starting service: {}", self.name() /* self.markets.len() */,);

        //let markets = self.markets.iter().map(|(key, _)| key.to_string()).collect::<Vec<_>>();
        //let filter = self.client_geyser.craft_filter(markets.clone()).await;
        //let mut stream = self.client_geyser.subscribe(filter).await;

        /*
         * I have to, in some way, get all the accounts we need to track, through Amm::accounts_to_update()
         * and map them to the appropriate Amm..right? .. so that once we receive an update, we can
         * deserialize and update the in-memory state of the Amm (through Amm::update()).
         */

        // defined on a per-market basis
        let amms: Vec<Box<dyn Amm>> = vec![
            Box::new(RaydiumCP::new()), // ..
            Box::new(RaydiumCP::new()),
            Box::new(ObricV2::new()),
            Box::new(ObricV2::new()),
        ];

        let tmp_program_id: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
        let tmp_market: Pubkey = pubkey!("34qhkhrhyNinGwRUfTQJNB2VFv8oQYqd5TvDwWDE1MaN");
        let acc = self.client_default.get_account(&tmp_market).await?;
        let keyed_account = KeyedAccount { key: tmp_market, account: acc, params: None };
        let acc_des = Box::new(RaydiumCP::from_keyed_account(&keyed_account, &AmmContext::default())?);
        info!("deserialised raydium CP acc: {:?}", acc_des);
        //
        // HashMap<Pubkey, Vec<Pubkey>>
        //   -> the key is the program (amm) addr
        //   -> the value is a list of the markets we collect data for
        //
        // HashMap<Pubkey, Pubkey>
        //   -> the key is the account that we follow for updates
        //   -> the value is the market addr
        //
        // HashMap<Pubkey, Account> (aka AccountMap)
        //   -> the key is the account that we follow for updates
        //   -> the value is the account
        //

        //let tmp_program_id = pubkey!("obriQD1zbpyLz95G5n7nJe6a4DPjpFwa5XYPoNm113y");
        //let tmp_market = pubkey!("BWBHrYqfcjAh5dSiRwzPnY4656cApXVXmkeDmAfwBKQG");
        //let acc = self.client_default.get_account(&tmp_market)?;
        //let keyed_account = KeyedAccount { key: tmp_market, account: acc, params: None };
        //let acc_des = Box::new(ObricV2::from_keyed_account(&keyed_account, &AmmContext::default())?);
        //info!("deserialised ObricV2Amm acc: {:?}", acc_des);

        let mut accs: HashMap<Pubkey, Box<dyn Amm>> = HashMap::new();
        accs.insert(tmp_market, acc_des.clone());

        let accs_to_update = acc_des.get_accounts_to_update();
        let accs_map_basic: HashMap<Pubkey, Pubkey> = accs_to_update.iter().map(|v| (*v, tmp_market)).collect();
        let accounts_map: AccountMap =
            accs_to_update.iter().zip(self.client_default.get_multiple_accounts(&accs_to_update).await?).map(|(key, acc)| (*key, acc.unwrap())).collect();

        let filter = self.client_geyser.craft_filter(accs_to_update.iter().map(|v| v.to_string()).collect()).await;
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

                        info!("pubkey: {:#?} | account: {:#?} | accs_map: {:#?} | accs: {:#?}", pubkey, account, accs_map_basic, accs);

                        let market = accs_map_basic.get(&pubkey).unwrap();
                        if let Some(amm) = accs.get_mut(market) {
                            info!("AMM ::: {:?}", amm);
                            match amm.update(&accounts_map) {
                                Ok(_) => {
                                    info!("updated state | {:?}", amm);
                                }
                                Err(err) => {
                                    error!("Failed to update AMM: {}", err);
                                }
                            }
                        }

                        // we'll hardcode the AMMs for now;

                        // we'll have to match the account against a particular amm before proceeding with a concrete
                        // deserialisation format
                        // then send a meaningful msg downstream towards a `impl Strategy`
                        //
                        // 1. deserialise in the proper AMM representation
                        // 2. all AMMs should be implementing the `AMM` trait
                        // 3. the message we'll pass down will be some kind of Vec<Box<dyn Amm>>
                        // 4. since all markets will have `Amm` implemented - we'll have a clear path to
                        //    get quotes & execute swaps — both will be done in the Solve thread.
                        //match account.owner {
                        //    SOLFI_V1 => {
                        //        info!("SOLFI_V1 | {:?} | {:?}", pubkey, account);
                        //    }
                        //    SOLFI_V2 => {
                        //        info!("SOLFI_V2 | {:?} | {:?}", pubkey, account);
                        //    }
                        //    OBRIC_V2 => {
                        //        info!("OBRIC_V2 | {:?} | {:?}", pubkey, account);
                        //    }
                        //    RAYDIUM_CP => {
                        //        info!("RAYDIUM_CP | {:?} | {:?}", pubkey, account);
                        //    }
                        //    RAYDIUM_CL => {
                        //        info!("RAYDIUM_CL | {:?} | {:?}", pubkey, account);
                        //    }
                        //    _ => {
                        //        unimplemented!("Market not yet supported - {}", pubkey);
                        //    }
                        //}
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
