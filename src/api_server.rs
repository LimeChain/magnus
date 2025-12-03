use std::str::FromStr;

use actix_web::{App, HttpResponse, HttpServer, middleware::Logger, web};
use magnus::adapters::{
    QuoteParams, QuoteResponse, SwapMode,
    aggregators::{Aggregator, dflow::DFlow, jupiter::Jupiter},
};
use metrics::counter;
use serde::Deserialize;
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use tracing_actix_web::TracingLogger;
use utoipa::{OpenApi, ToSchema};
use utoipa_rapidoc::RapiDoc;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Copy, Clone, Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SrcKind {
    // get the best pricing from any aggregator
    #[default]
    Aggregators,

    // poke only one of the aggregators for a price
    Jupiter,
    DFlow,

    // get the best pricing from any of the integrated AMMs
    // perhaps we can get even more granular here and segment into (prop|public) AMMs
    #[serde(rename = "amms")]
    AMMs,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuoteOrSimParam {
    input_mint: String,
    output_mint: String,
    amount: u64,

    #[serde(default)]
    src_kind: SrcKind,
}
// --

#[derive(Debug, Clone)]
pub struct ApiServerCfg {
    pub host: String,
    pub workers: u16,
}

pub struct ApiServer {
    inner: actix_web::dev::Server,
}

impl ApiServer {
    pub fn new(cfg: ApiServerCfg) -> eyre::Result<ApiServer> {
        #[derive(Copy, Clone, OpenApi)]
        #[openapi(paths(quote_handler, simulate_handler))]
        struct ApiDoc;
        let openapi = ApiDoc::openapi();

        Ok(ApiServer {
            inner: HttpServer::new(move || {
                App::new()
                    // middlewares
                    .wrap(Logger::default())
                    .wrap(TracingLogger::default())

                    // routes - docs
                    .service(RapiDoc::with_openapi("docs/openapi.json", openapi.clone()).path("/docs"))
                    .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/swagger-ui/openapi.json", openapi.clone()))

                    // routes - api
                    .route("/health", web::get().to(HttpResponse::Ok))
                    .service(
                        web::scope("/api").service(
                            web::scope("/v1")
                                // trading related
                                .route("/quote", web::get().to(quote_handler))
                                .route("/simulate", web::get().to(simulate_handler))

                                .route("/markets/supported", web::get().to(|| async { HttpResponse::NotImplemented().finish() })) // analytics?
                                .route("/markets/load", web::get().to(|| async { HttpResponse::NotImplemented().finish() })) // hotload new markets?
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

fn sanity_check_quote_or_sim_param(params: &QuoteOrSimParam) -> eyre::Result<(Pubkey, Pubkey)> {
    // sanity check the mints are actual valid pubkeys
    let keys = match (Pubkey::from_str(&params.input_mint).is_err(), Pubkey::from_str(&params.output_mint).is_err()) {
        (true, true) => eyre::bail!("Invalid inputMint and outputMint"),
        (true, _) => eyre::bail!("Invalid inputMint"),
        (_, true) => eyre::bail!("Invalid outputMint"),
        _ => (Pubkey::from_str(&params.input_mint)?, Pubkey::from_str(&params.output_mint)?),
    };

    Ok(keys)
}

#[utoipa::path(
    get,
    path = "/api/v1/quote",
    params(
        ("inputMint" = String, description = "The input token mint addr"),
        ("outputMint" = String, description = "The output token mint addr"),
        ("amount" = u64, description = "The amount to quote")
    ),
    responses(
        (status = 200, description = "Successfully retrieved the quote", body = QuoteResponse),
        (status = 500, description = "Internal Server Error")
    )
)]
async fn quote_handler(params: web::Query<QuoteOrSimParam>) -> HttpResponse {
    counter!("API HITS", "quotes" => "/api/v1/quote").increment(1);

    let (input_mint, output_mint) = match sanity_check_quote_or_sim_param(&params) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(keys) => keys,
    };

    match params.src_kind {
        SrcKind::Aggregators => {
            let aggregators: Vec<Box<dyn Aggregator>> = vec![Box::new(Jupiter {}), Box::new(DFlow {})];
            let quote_param = QuoteParams { input_mint, output_mint, amount: params.amount, swap_mode: SwapMode::ExactIn };

            let handles: Vec<_> = aggregators.into_iter().map(|agg| tokio::spawn(async move { agg.quote(&quote_param.clone()).await })).collect();
            let res = futures::future::join_all(handles).await;
            let best_quote = res.into_iter().filter_map(|handle_result| handle_result.ok().and_then(|quote_result| quote_result.ok())).max_by_key(|quote| quote.out_amount);

            match best_quote {
                Some(quote) => HttpResponse::Ok().json(quote),
                None => HttpResponse::InternalServerError().json(json!({"error": "err acquiring aggregators market data"})),
            }
        }
        SrcKind::Jupiter => {
            let param = QuoteParams { input_mint, output_mint, amount: params.amount, swap_mode: SwapMode::ExactIn };

            match (Jupiter {}.quote(&param).await) {
                Ok(jup) => HttpResponse::Ok().json(jup),
                Err(err) => HttpResponse::InternalServerError().json(json!({"error": err.to_string()})),
            }
        }
        SrcKind::DFlow => {
            let param = QuoteParams { input_mint, output_mint, amount: params.amount, swap_mode: SwapMode::ExactIn };

            match (DFlow {}.quote(&param).await) {
                Ok(dflow) => HttpResponse::Ok().json(dflow),
                Err(err) => HttpResponse::InternalServerError().json(json!({"error": err.to_string()})),
            }
        }
        SrcKind::AMMs => {
            // TODO; - we'll send a msg towards `Solve::compute`
            // and based on the provided result, we'll return the appropriate response
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
pub async fn simulate_handler(params: web::Query<QuoteOrSimParam>) -> HttpResponse {
    counter!("API HITS", "simulations" => "/api/v1/simulate").increment(1);

    if let Err(e) = sanity_check_quote_or_sim_param(&params) {
        return HttpResponse::BadRequest().body(e.to_string());
    }

    HttpResponse::NotImplemented().finish()
}
