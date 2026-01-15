use futures_util::StreamExt as _;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use tracing::{error, info};
use yellowstone_grpc_client::{GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::geyser::subscribe_update;

use crate::{AccountMap, Ingest, IngestCtx, Markets, StateAccountToMarket, geyser_client::GeyserClientWrapped, helpers::geyser_acc_to_native};

pub struct IngestorCfg<T: Interceptor + Send + Sync> {
    pub client_geyser: GeyserGrpcClient<T>,
    pub client_default: std::sync::Arc<RpcClient>,
    pub markets: Markets,
    pub account_map: AccountMap,
}

pub struct GeyserPoolStateIngestor<T: Interceptor + Send + Sync> {
    client_geyser: GeyserClientWrapped<T>,
    _client_default: std::sync::Arc<RpcClient>,
    markets: Markets,
    account_map: AccountMap,
}

impl<T: Interceptor + Send + Sync> GeyserPoolStateIngestor<T> {
    pub fn new(cfg: IngestorCfg<T>) -> Self {
        Self { client_geyser: GeyserClientWrapped::new(cfg.client_geyser), _client_default: cfg.client_default, markets: cfg.markets, account_map: cfg.account_map }
    }
}

#[async_trait::async_trait]
impl<T: Interceptor + Send + Sync> Ingest for GeyserPoolStateIngestor<T> {
    fn name(&self) -> &str {
        "GeyserPoolStateIngestor"
    }

    async fn ingest<C: IngestCtx>(&mut self, _: C) -> eyre::Result<()> {
        info!("starting service: {}", self.name());

        let state_acc_to_market: StateAccountToMarket = self
            .markets
            .lock()
            .unwrap()
            .values()
            .flat_map(|market| {
                let accs = market.get_accounts_to_update();
                accs.into_iter().map(|acc| (acc, market.key()))
            })
            .collect();

        let filter = self.client_geyser.craft_filter(state_acc_to_market.keys().map(|v| v.to_string()).collect()).await;
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
                        let slot = account_update.slot;
                        self.account_map.insert(pubkey, account);

                        // we don't need to send a msg to `Strategy` since we're sharing the underlying structure
                        let market_pubkey = state_acc_to_market.get(&pubkey).unwrap();
                        if let Some(market) = self.markets.lock().unwrap().get_mut(market_pubkey)
                            && let Ok(_) = market.update(&self.account_map, Some(slot))
                        {
                            info!("recv update for market: {:?}", market);
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
