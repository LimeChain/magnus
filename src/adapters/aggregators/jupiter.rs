use serde::{Deserialize, Serialize};

use crate::{
    adapters::{Adapter, AggregatorKind, QuotePlanItem, QuoteResponse, aggregators::Aggregator},
    helpers::parse_amount,
};

const API_URL: &str = "https://quote-api.jup.ag";

pub struct Jupiter;

impl Adapter for Jupiter {}

#[async_trait::async_trait]
impl Aggregator for Jupiter {
    async fn quote(&self, params: &crate::adapters::QuoteParams) -> eyre::Result<crate::adapters::QuoteResponse> {
        let url = format!("{}/ultra/v1/order?inputMint={}&outputMint={}&amount={}", API_URL, params.input_mint, params.output_mint, params.amount);

        let resp: JupiterQuoteResp = reqwest::get(&url).await?.json().await?;
        let quote = crate::adapters::QuoteResponse::from(resp);

        Ok(quote)
    }

    async fn swap(&self, _params: &crate::adapters::SwapParams) -> eyre::Result<crate::adapters::SwapAndAccountMetas> {
        unimplemented!("not yet implemented")
    }
}

//#[async_trait::async_trait]
//impl Aggregator for Jupiter {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JupSwapInfo {
    pub amm_key: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JupRoutePlanItem {
    pub swap_info: JupSwapInfo,
    pub percent: Option<u8>,
    pub bps: Option<u16>,
    pub usd_value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterQuoteResp {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub swap_usd_value: Option<f64>,
    pub route_plan: Vec<JupRoutePlanItem>,
}

impl From<JupiterQuoteResp> for QuoteResponse {
    fn from(jup: JupiterQuoteResp) -> Self {
        let route_plan = jup
            .route_plan
            .iter()
            .map(|v| QuotePlanItem {
                venue: v.swap_info.label.clone(),
                market_key: v.swap_info.amm_key.clone(),
                input_mint: v.swap_info.input_mint.clone(),
                output_mint: v.swap_info.output_mint.clone(),
                in_amount: parse_amount(&v.swap_info.in_amount).unwrap_or(0),
                out_amount: parse_amount(&v.swap_info.out_amount).unwrap_or(0),
            })
            .collect();

        QuoteResponse {
            aggregator: AggregatorKind::Jupiter,
            input_mint: jup.input_mint,
            output_mint: jup.output_mint,
            in_amount: parse_amount(&jup.in_amount).unwrap_or(0),
            out_amount: parse_amount(&jup.out_amount).unwrap_or(0),
            route_plan: Some(route_plan),
        }
    }
}
