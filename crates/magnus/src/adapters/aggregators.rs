pub mod dflow;
pub mod jupiter;

use crate::adapters::Adapter;

#[async_trait::async_trait]
pub trait Aggregator: Adapter + Send + Sync {
    async fn quote(&self, _params: &crate::adapters::QuoteParams) -> eyre::Result<crate::adapters::IntQuoteResponse>;
    async fn swap(&self, _params: &crate::adapters::SwapParams) -> eyre::Result<crate::adapters::SwapAndAccountMetas>;
}
