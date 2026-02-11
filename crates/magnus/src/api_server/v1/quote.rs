use std::str::FromStr;

use actix_web::{HttpResponse, web};
#[cfg(feature = "metrics")]
use metrics::counter;
use serde::Deserialize;
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use utoipa::ToSchema;

use crate::{
    adapters::{
        IntQuoteResponse, QuoteParams, SwapMode,
        aggregators::{Aggregator, dflow::DFlow, jupiter::Jupiter},
        amms::Target,
    },
    api_server::ServerState,
    strategy::{DispatchParams, DispatchResponse},
};

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuoteUserParam {
    input_mint: String,
    output_mint: String,
    amount: u64,

    #[serde(default)]
    target: Target,
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
        (status = 200, description = "Successfully retrieved the quote", body = IntQuoteResponse),
        (status = 500, description = "Internal Server Error")
    )
)]
pub async fn quote_handler(params: web::Query<QuoteUserParam>, state: web::Data<ServerState>) -> HttpResponse {
    #[cfg(feature = "metrics")]
    counter!("API HITS", "quotes" => "/api/v1/quote").increment(1);

    let (input_mint, output_mint) = match sanity_check_quote_param(&params) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(keys) => keys,
    };

    match params.target {
        Target::Aggregators => {
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
        Target::Jupiter => {
            let param = QuoteParams { input_mint, output_mint, amount: params.amount, swap_mode: SwapMode::ExactIn };

            match (Jupiter {}.quote(&param).await) {
                Ok(jup) => HttpResponse::Ok().json(jup),
                Err(err) => HttpResponse::InternalServerError().json(json!({"error": err.to_string()})),
            }
        }
        Target::DFlow => {
            let param = QuoteParams { input_mint, output_mint, amount: params.amount, swap_mode: SwapMode::ExactIn };

            match (DFlow {}.quote(&param).await) {
                Ok(dflow) => HttpResponse::Ok().json(dflow),
                Err(err) => HttpResponse::InternalServerError().json(json!({"error": err.to_string()})),
            }
        }
        Target::AMMs => {
            let (response_tx, response_rx) = oneshot::channel::<DispatchResponse>();

            let dispatch = DispatchParams::Quote { params: QuoteParams { swap_mode: SwapMode::ExactIn, amount: params.amount, input_mint, output_mint }, response_tx };

            state.request_tx.send(dispatch).expect("send invalid transmitter req");
            tracing::info!("sent from `API Server::quote` towards `Strategy`");
            let response = response_rx.recv();
            tracing::info!("received from `Strategy`");

            match response {
                Ok(response) => HttpResponse::Ok().json(response),
                Err(_) => HttpResponse::InternalServerError().json(json!({"error": "no response"})),
            }
        }
    }
}

fn sanity_check_quote_param(params: &QuoteUserParam) -> eyre::Result<(Pubkey, Pubkey)> {
    // sanity check the mints are actual valid pubkeys
    let keys = match (Pubkey::from_str(&params.input_mint).is_err(), Pubkey::from_str(&params.output_mint).is_err()) {
        (true, true) => eyre::bail!("Invalid inputMint and outputMint"),
        (true, _) => eyre::bail!("Invalid inputMint"),
        (_, true) => eyre::bail!("Invalid outputMint"),
        _ => (Pubkey::from_str(&params.input_mint)?, Pubkey::from_str(&params.output_mint)?),
    };

    Ok(keys)
}
