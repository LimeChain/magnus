use std::str::FromStr;

use actix_web::{App, HttpResponse, HttpServer, web};
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use utoipa::{OpenApi, ToSchema};
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

const JUPITER_BASE_URL: &str = "https://lite-api.jup.ag";
const DFLOW_BASE_URL: &str = "https://quote-api.dflow.net";

#[derive(Debug, Clone)]
pub struct ApiServerCfg {
    pub host: String,
    pub workers: u16,
}

pub struct ApiServer {
    inner: actix_web::dev::Server,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Aggregators {
    #[default]
    Jupiter,
    DFlow,
}

#[derive(Copy, Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SrcKind {
    // get the best pricing from any aggregator
    Aggregators,

    // poke only one of the aggregators for a price
    Jupiter,
    DFlow,

    // get the best pricing from any of the integrated AMMs
    // perhaps we can get even more granular here and segment into (prop|public) AMMs
    #[serde(rename = "amms")]
    AMMs,
}

impl Default for SrcKind {
    fn default() -> Self {
        SrcKind::Aggregators
    }
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuoteParam {
    input_mint: String,
    output_mint: String,
    amount: u64,

    #[serde(default)]
    src_kind: SrcKind,
}

// -- jupiter
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

// Nested structure for a single Jupiter route plan entry (UPDATED)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JupRoutePlanItem {
    pub swap_info: JupSwapInfo,
    pub percent: Option<u8>,
    pub bps: Option<u16>,
    pub usd_value: f64,
}

// Full Jupiter Quote Response struct (UPDATED)
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
// --

// -- dflow
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
// --

// -- internal
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct QuotePlanItem {
    pub venue: String,
    pub market_key: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct QuoteResponse {
    pub aggregator: Aggregators,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
    pub route_plan: Vec<QuotePlanItem>,
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
            aggregator: Aggregators::Jupiter,
            input_mint: jup.input_mint,
            output_mint: jup.output_mint,
            in_amount: parse_amount(&jup.in_amount).unwrap_or(0),
            out_amount: parse_amount(&jup.out_amount).unwrap_or(0),
            route_plan,
        }
    }
}

impl From<DFlowQuoteResponse> for QuoteResponse {
    fn from(dflow: DFlowQuoteResponse) -> Self {
        let route_plan = dflow
            .route_plan
            .iter()
            .map(|v| QuotePlanItem {
                venue: v.venue.clone(),
                market_key: v.market_key.clone(),
                input_mint: v.input_mint.clone(),
                output_mint: v.output_mint.clone(),
                in_amount: parse_amount(&v.in_amount).unwrap_or(0),
                out_amount: parse_amount(&v.out_amount).unwrap_or(0),
            })
            .collect();

        QuoteResponse {
            aggregator: Aggregators::DFlow,
            input_mint: dflow.input_mint,
            output_mint: dflow.output_mint,
            in_amount: parse_amount(&dflow.in_amount).unwrap_or(0),
            out_amount: parse_amount(&dflow.out_amount).unwrap_or(0),
            route_plan,
        }
    }
}

#[derive(OpenApi)]
#[openapi(paths(quote_handler, simulate_handler))]
struct ApiDoc;

impl ApiServer {
    pub fn new(cfg: ApiServerCfg) -> eyre::Result<ApiServer> {
        let openapi = ApiDoc::openapi();

        Ok(ApiServer {
            inner: HttpServer::new(move || {
                App::new()
                    // routes
                    .service(Redoc::with_url("/redoc", openapi.clone()))
                    .service(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
                    .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()))

                    .route("/health", web::get().to(HttpResponse::Ok))
                    .service(
                        web::scope("/api").service(
                            web::scope("/v1")
                                // trading related
                                .route("/quote", web::get().to(quote_handler))
                                .route("/simulate", web::get().to(simulate_handler))

                                .route("/markets/supported", web::get().to(|| async { HttpResponse::NotImplemented().finish() })) // analytics?
                                .route("/markets/load", web::get().to(|| async { HttpResponse::NotImplemented().finish() })) // hotload new markets
                            )
                    )
            })
            .workers(cfg.workers as usize)
            .bind(cfg.host.as_str())?
            .disable_signals()
            .run(),
        })
    }

    pub async fn start(self) -> std::io::Result<()> {
        self.inner.await
    }
}

fn parse_amount(s: &str) -> Option<u64> {
    s.parse::<u64>().ok()
}

async fn jup_quote(input_mint: &String, output_mint: &String, amount: u64) -> eyre::Result<QuoteResponse> {
    let url = format!("{}/ultra/v1/order?inputMint={}&outputMint={}&amount={}", JUPITER_BASE_URL, input_mint, output_mint, amount);
    let resp: JupiterQuoteResp = reqwest::get(&url).await?.json().await?;
    let parsed = QuoteResponse::from(resp);

    Ok(parsed)
}

async fn dflow_quote(input_mint: &String, output_mint: &String, amount: u64) -> eyre::Result<QuoteResponse> {
    let url = format!("{}/quote?inputMint={}&outputMint={}&amount={}", DFLOW_BASE_URL, input_mint, output_mint, amount);
    let resp: DFlowQuoteResponse = reqwest::get(&url).await?.json().await?;
    let parsed = QuoteResponse::from(resp);

    Ok(parsed)
}

#[utoipa::path(
    get,
    path = "/api/v1/quote",
    params(
        ("input_mint" = String, description = "The input token mint addr"),
        ("output_mint" = String, description = "The output token mint addr"),
        ("amount" = u64, description = "The amount to quote")
    ),
    responses(
        (status = 200, description = "Successfully retrieved the quote", body = QuoteResponse),
        (status = 500, description = "Internal Server Error")
    )
)]
async fn quote_handler(params: web::Query<QuoteParam>) -> HttpResponse {
    // sanity check the mints are actual valid pubkeys
    match (Pubkey::from_str(&params.input_mint).is_err(), Pubkey::from_str(&params.output_mint).is_err()) {
        (true, true) => return HttpResponse::BadRequest().body("Invalid input_mint and output_mint"),
        (true, _) => return HttpResponse::BadRequest().body("Invalid input_mint"),
        (_, true) => return HttpResponse::BadRequest().body("Invalid output_mint"),
        _ => {}
    }

    match params.src_kind {
        SrcKind::Aggregators => {
            let (input_mint_jup, output_mint_jup, amount_jup) = (params.input_mint.clone(), params.output_mint.clone(), params.amount);
            let (input_mint_dflow, output_mint_dflow, amount_dflow) = (params.input_mint.clone(), params.output_mint.clone(), params.amount);

            // spawn separate tasks for JUP and DFlow, and then await their concurrent exec
            let jup_handle = tokio::spawn(async move { jup_quote(&input_mint_jup, &output_mint_jup, amount_jup).await });
            let dflow_handle = tokio::spawn(async move { dflow_quote(&input_mint_dflow, &output_mint_dflow, amount_dflow).await });

            let (jup_result, dflow_result) = tokio::join!(jup_handle, dflow_handle);
            let (jup_result, dflow_result) = (jup_result.expect("jup err-ed out"), dflow_result.expect("dflow err-ed out"));

            // check jup & dflow
            match (jup_result, dflow_result) {
                (Ok(jup), Ok(dflow)) => match jup.out_amount > dflow.out_amount {
                    true => HttpResponse::Ok().json(jup),
                    false => HttpResponse::Ok().json(dflow),
                },
                (Ok(jup), _) => HttpResponse::Ok().json(jup),
                (_, Ok(dflow)) => HttpResponse::Ok().json(dflow),
                (_, _) => HttpResponse::InternalServerError().json(json!({"error": "err acquiring aggregators — jupiter & dflow — market data"})),
            }
        }
        SrcKind::Jupiter => {
            let r = jup_quote(&params.input_mint, &params.output_mint, params.amount).await.expect("jup call err-ed out");
            HttpResponse::Ok().json(r)
        }
        SrcKind::DFlow => {
            let r = dflow_quote(&params.input_mint, &params.output_mint, params.amount).await.expect("dflow call err-ed out");
            HttpResponse::Ok().json(r)
        }
        SrcKind::AMMs => {
            // TODO;
            HttpResponse::NotImplemented().finish()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/simulate",
    responses(
        (status = 200, description = "Simulation successful"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn simulate_handler() -> HttpResponse {
    HttpResponse::NotImplemented().finish()
}
