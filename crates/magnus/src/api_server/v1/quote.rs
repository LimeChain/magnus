use actix_web::{HttpResponse, web};
#[cfg(feature = "metrics")]
use metrics::counter;
use serde_json::json;

use crate::{
    adapters::{
        QuoteAndSwapResponse, QuoteParams, SwapMode,
        aggregators::{Aggregator, dflow::DFlow, jupiter::Jupiter},
        amms::LiquiditySource,
    },
    api_server::{QuoteOrSwapUserParam, ServerState, sanity_check_quote_or_sim_param},
    strategy::DispatchParams,
};

#[utoipa::path(
    get,
    path = "/api/v1/quote",
    params(
        ("inputMint" = String, description = "The input token mint addr"),
        ("outputMint" = String, description = "The output token mint addr"),
        ("amount" = u64, description = "The amount to quote")
    ),
    responses(
        (status = 200, description = "Successfully retrieved the quote", body = QuoteAndSwapResponse),
        (status = 500, description = "Internal Server Error")
    )
)]
pub async fn quote_handler(params: web::Query<QuoteOrSwapUserParam>, state: web::Data<ServerState>) -> HttpResponse {
    #[cfg(feature = "metrics")]
    counter!("API HITS", "quotes" => "/api/v1/quote").increment(1);

    let (input_mint, output_mint) = match sanity_check_quote_or_sim_param(&params) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(keys) => keys,
    };

    match params.src_kind {
        LiquiditySource::Aggregators => {
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
        LiquiditySource::Jupiter => {
            let param = QuoteParams { input_mint, output_mint, amount: params.amount, swap_mode: SwapMode::ExactIn };

            match (Jupiter {}.quote(&param).await) {
                Ok(jup) => HttpResponse::Ok().json(jup),
                Err(err) => HttpResponse::InternalServerError().json(json!({"error": err.to_string()})),
            }
        }
        LiquiditySource::DFlow => {
            let param = QuoteParams { input_mint, output_mint, amount: params.amount, swap_mode: SwapMode::ExactIn };

            match (DFlow {}.quote(&param).await) {
                Ok(dflow) => HttpResponse::Ok().json(dflow),
                Err(err) => HttpResponse::InternalServerError().json(json!({"error": err.to_string()})),
            }
        }
        LiquiditySource::AMMs => {
            // TODO; - we'll send a msg towards `Solve::compute`
            // and based on the provided result, we'll return the appropriate response
            let (response_tx, response_rx) = oneshot::channel();

            let dispatch = DispatchParams::Quote { params: QuoteParams { swap_mode: SwapMode::ExactIn, amount: params.amount, input_mint, output_mint }, response_tx };

            state.request_tx.send(dispatch).expect("send invalid transmitter req");
            let response = response_rx.recv();

            match response {
                Ok(response) => HttpResponse::Ok().json(response),
                Err(_) => HttpResponse::InternalServerError().json(json!({"error": "no response"})),
            }
        }
    }
}
