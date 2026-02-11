use std::str::FromStr;

use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signature;

use crate::{
    adapters::{IntQuoteResponse, PlanItem, QuoteParams, amms::Target},
    helpers::parse_amount,
};

const API_URL: &str = "https://lite-api.jup.ag/swap/v1/quote";
const DEX_FILTER: &str = "AlphaQ";

pub struct AlphaQ;

impl AlphaQ {
    pub async fn quote(params: &QuoteParams) -> eyre::Result<IntQuoteResponse> {
        let url = format!("{API_URL}?inputMint={}&outputMint={}&amount={}&dexes={DEX_FILTER}", params.input_mint, params.output_mint, params.amount);
        let body = reqwest::get(&url).await?.text().await?;

        let response: AlphaQApiResponse = serde_json::from_str(&body)?;
        match response {
            AlphaQApiResponse::Quote(quote) => Ok(IntQuoteResponse::from(quote)),
            AlphaQApiResponse::Error(err) => eyre::bail!("alphaq quote error: {}", err.error),
        }
    }
}

pub fn parse_signature(signature: &str) -> eyre::Result<Signature> {
    Signature::from_str(signature).map_err(Into::into)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum AlphaQApiResponse {
    Quote(AlphaQQuoteResponse),
    Error(AlphaQErrorResponse),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlphaQErrorResponse {
    error: String,
    #[allow(dead_code)]
    error_code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlphaQSwapInfo {
    pub amm_key: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    #[allow(dead_code)]
    pub out_amount_after_slippage: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlphaQRoutePlanItem {
    pub swap_info: AlphaQSwapInfo,
    pub percent: Option<u8>,
    pub bps: Option<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlphaQQuoteResponse {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub route_plan: Vec<AlphaQRoutePlanItem>,
}

impl From<AlphaQQuoteResponse> for IntQuoteResponse {
    fn from(alphaq: AlphaQQuoteResponse) -> Self {
        let route_plan = alphaq
            .route_plan
            .iter()
            .map(|v| PlanItem {
                venue: v.swap_info.label.clone(),
                market_key: v.swap_info.amm_key.clone(),
                input_mint: v.swap_info.input_mint.clone(),
                output_mint: v.swap_info.output_mint.clone(),
                in_amount: parse_amount(&v.swap_info.in_amount).unwrap_or(0),
                out_amount: parse_amount(&v.swap_info.out_amount).unwrap_or(0),
            })
            .collect();

        IntQuoteResponse {
            source: Target::AlphaQ,
            input_mint: alphaq.input_mint,
            output_mint: alphaq.output_mint,
            in_amount: parse_amount(&alphaq.in_amount).unwrap_or(0),
            out_amount: parse_amount(&alphaq.out_amount).unwrap_or(0),
            route_plan: Some(route_plan),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_quote_response() -> serde_json::Value {
        serde_json::json!({
            "inputMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "inAmount": "1000000",
            "outputMint": "USD1ttGY1N17NEEHLmELoaybftRBUSErhqYiQzvEmuB",
            "outAmount": "1000330",
            "routePlan": [{
                "swapInfo": {
                    "ammKey": "9xPhpwq6GLUkrDBNfXCbnSP9ARAMMyUQqgkrqaDW6NLV",
                    "label": "AlphaQ",
                    "inputMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                    "outputMint": "USD1ttGY1N17NEEHLmELoaybftRBUSErhqYiQzvEmuB",
                    "inAmount": "1000000",
                    "outAmount": "1000330",
                    "outAmountAfterSlippage": "1000330"
                },
                "percent": 100,
                "bps": null
            }]
        })
    }

    #[test]
    fn alphaq_quote_response_into_int_quote_response() {
        let quote = serde_json::from_value::<AlphaQQuoteResponse>(sample_quote_response()).expect("sample quote should deserialize");
        let mapped = IntQuoteResponse::from(quote);

        assert!(matches!(mapped.source, Target::AlphaQ));
        assert_eq!(mapped.in_amount, 1_000_000);
        assert_eq!(mapped.out_amount, 1_000_330);
        assert_eq!(mapped.route_plan.as_ref().expect("route plan should exist").len(), 1);
        assert_eq!(mapped.route_plan.expect("route plan should exist")[0].venue, "AlphaQ");
    }

    #[test]
    fn parse_signature_accepts_valid_signature() {
        let signature = "3eFv5uKc6bPFuFauLmMiyBGRZvvhDie1F3b6xkU3zuPWYCCJ5BVeS5EkVuZkwBFJJ1xsjLJkTVfBNU1o5WBUsbrf";
        let parsed = parse_signature(signature).expect("signature should parse");
        assert_eq!(parsed.to_string(), signature);
    }

    #[test]
    fn parse_signature_rejects_invalid_signature() {
        let invalid = "alphaq";
        assert!(parse_signature(invalid).is_err());
    }
}
