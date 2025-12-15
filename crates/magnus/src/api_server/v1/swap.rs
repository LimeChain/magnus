use actix_web::{HttpResponse, web};
use metrics::counter;

use crate::api_server::{QuoteOrSwapUserParam, ServerState, sanity_check_quote_or_sim_param};

#[utoipa::path(
    post,
    path = "/api/v1/swap",
    responses(
        (status = 200, description = "Swap successful"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn swap_handler(params: web::Query<QuoteOrSwapUserParam>, _: web::Data<ServerState>) -> HttpResponse {
    counter!("API HITS", "swaps" => "/api/v1/swap").increment(1);

    if let Err(e) = sanity_check_quote_or_sim_param(&params) {
        return HttpResponse::BadRequest().body(e.to_string());
    }

    HttpResponse::NotImplemented().finish()
}
