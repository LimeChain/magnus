use std::collections::HashMap;

use futures::Stream;
use tonic::Status;
use yellowstone_grpc_client::{GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::{
    geyser::{SubscribeRequest, SubscribeRequestFilterAccounts},
    prelude::SubscribeUpdate,
};

pub struct GeyserClientWrapped<F: Interceptor> {
    inner: GeyserGrpcClient<F>,
}

impl<F: Interceptor> GeyserClientWrapped<F> {
    pub fn new(inner: GeyserGrpcClient<F>) -> Self {
        Self { inner }
    }

    pub async fn craft_filter(&self, accounts: Vec<String>) -> SubscribeRequest {
        let accounts_filter = SubscribeRequestFilterAccounts { account: accounts, owner: vec![], filters: vec![], nonempty_txn_signature: Some(true) };

        let mut filter = HashMap::new();
        filter.insert("client".to_string(), accounts_filter).unwrap_or_default();

        SubscribeRequest { accounts: filter, ..Default::default() }
    }

    pub async fn subscribe(&mut self, sub_request: SubscribeRequest) -> impl Stream<Item = Result<SubscribeUpdate, Status>> {
        let (_, stream) = self.inner.subscribe_with_request(Some(sub_request)).await.expect("unable to subscribe to geyser");

        stream
    }
}
