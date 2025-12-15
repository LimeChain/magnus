use serde::{Deserialize, Serialize};

use crate::{
    adapters::{Adapter, PlanItem, QuoteAndSwapResponse, QuoteParams, aggregators::Aggregator, amms::LiquiditySource},
    helpers::parse_amount,
};

const API_URL: &str = "https://quote-api.dflow.net";

pub struct DFlow;

impl Adapter for DFlow {}

#[async_trait::async_trait]
impl Aggregator for DFlow {
    async fn quote(&self, params: &QuoteParams) -> eyre::Result<crate::adapters::QuoteAndSwapResponse> {
        let url = format!("{}/quote?inputMint={}&outputMint={}&amount={}", API_URL, params.input_mint, params.output_mint, params.amount);

        let resp: DFlowQuoteResponse = reqwest::get(&url).await?.json().await?;
        let quote = QuoteAndSwapResponse::from(resp);

        Ok(quote)
    }

    async fn swap(&self, _params: &crate::adapters::SwapParams) -> eyre::Result<crate::adapters::SwapAndAccountMetas> {
        unimplemented!("not yet implemented")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DFlowRoutePlanItem {
    pub venue: String,
    pub market_key: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DFlowQuoteResponse {
    pub input_mint: String,
    pub in_amount: String,
    pub output_mint: String,
    pub out_amount: String,
    pub route_plan: Vec<DFlowRoutePlanItem>,
}

impl From<DFlowQuoteResponse> for QuoteAndSwapResponse {
    fn from(dflow: DFlowQuoteResponse) -> Self {
        let route_plan = Some(
            dflow
                .route_plan
                .iter()
                .map(|v| PlanItem {
                    venue: v.venue.clone(),
                    market_key: v.market_key.clone(),
                    input_mint: v.input_mint.clone(),
                    output_mint: v.output_mint.clone(),
                    in_amount: parse_amount(&v.in_amount).unwrap_or(0),
                    out_amount: parse_amount(&v.out_amount).unwrap_or(0),
                })
                .collect(),
        );

        QuoteAndSwapResponse {
            source: LiquiditySource::DFlow,
            input_mint: dflow.input_mint,
            output_mint: dflow.output_mint,
            in_amount: parse_amount(&dflow.in_amount).unwrap_or(0),
            out_amount: parse_amount(&dflow.out_amount).unwrap_or(0),
            route_plan,
        }
    }
}
