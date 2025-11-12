pub mod dflow;
pub mod jupiter;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::adapters::Adapter;

/// ..
#[async_trait::async_trait]
pub trait Aggregator: Adapter {
    async fn quote(&self, _params: &crate::adapters::QuoteParams) -> eyre::Result<crate::adapters::QuoteResponse>;
    async fn swap(&self, _params: &crate::adapters::SwapParams) -> eyre::Result<crate::adapters::SwapAndAccountMetas>;
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AggregatorKind {
    #[default]
    Jupiter,
    DFlow,
}
