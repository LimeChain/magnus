use futures_util::StreamExt as _;
use solana_sdk::pubkey::Pubkey;
use tracing::error;
use yellowstone_grpc_client::{GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::geyser::subscribe_update;

use crate::{
    TransmitState, error,
    geyser_client::GeyserClientWrapped,
    helpers::{deserialize_anchor_account, geyser_acc_to_native},
};

/// ..
pub trait Ingest {
    fn name(&self) -> &str;

    fn ingest<T: TransmitState>(&mut self, state: T) -> eyre::Result<()>;
}

pub struct GeyserPoolStateIngestor<T: Interceptor> {
    client_geyser: GeyserClientWrapped<T>,
    accounts: Vec<String>,
}

impl<T: Interceptor> GeyserPoolStateIngestor<T> {
    pub fn new(client_geyser: GeyserGrpcClient<T>, accounts: Vec<String>) -> Self {
        Self { client_geyser: GeyserClientWrapped::new(client_geyser), accounts }
    }
}

impl<T: Interceptor> Ingest for GeyserPoolStateIngestor<T> {
    fn name(&self) -> &str {
        "GeyserPoolStateIngestor"
    }

    async fn ingest(&mut self, state: &T) -> eyre::Result<()> {
        let filter = self.client_geyser.craft_filter(self.accounts.clone()).await;
        let mut stream = self.client_geyser.subscribe(filter).await;

        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    // Handle the SubscribeUpdate
                    if let Some(update) = msg.update_oneof
                        && let subscribe_update::UpdateOneof::Account(account_update) = update
                        && let Some(account_info) = account_update.account
                    {
                        let pubkey = Pubkey::try_from(account_info.pubkey.as_slice()).expect("Invalid pubkey");
                        let account = geyser_acc_to_native(&account_info);

                        // we'll have to match the account against a particular amm before proceeding with a concrete
                        // deserialisation format
                        // then send a meaningful msg downstream towards a `impl Strategy`
                    }
                }
                Err(e) => {
                    error!("received unsupported message - {}", e);
                }
            }
        }

        Ok(())
    }
}
