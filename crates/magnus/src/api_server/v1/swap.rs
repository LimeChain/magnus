use actix_web::{HttpResponse, web};
#[cfg(feature = "metrics")]
use metrics::counter;
use serde::Deserialize;
use serde_json::json;
use solana_sdk::signature::Keypair;
use tracing::info;
use utoipa::ToSchema;

use crate::{
    adapters::{SwapParams, amms::Target},
    api_server::ServerState,
    strategy::{DispatchParams, DispatchResponse},
};

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwapUserParam {
    _input_mint: String,
    _output_mint: String,
    _amount: u64,
    _min_amount_out: Option<u64>,
    privkey: String,

    #[serde(default)]
    target: Target,
}

pub fn sanity_check_swap_param(_: &SwapUserParam) -> eyre::Result<()> {
    Ok(())
}

#[utoipa::path(
    post,
    path = "/api/v1/swap",
    responses(
        (status = 200, description = "Swap successful"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn swap_handler(params: web::Json<SwapUserParam>, state: web::Data<ServerState>) -> HttpResponse {
    #[cfg(feature = "metrics")]
    counter!("API HITS", "swaps" => "/api/v1/swap").increment(1);

    if let Err(e) = sanity_check_swap_param(&params) {
        return HttpResponse::BadRequest().json(json!({"error": e.to_string()}));
    }

    let keypair = match read_keypair(&params.privkey) {
        Ok(k) => k,
        Err(_) => return HttpResponse::BadRequest().json(json!({"error": "invalid private key"})),
    };

    info!("{:?}", keypair);

    match params.target {
        Target::Aggregators | Target::Jupiter | Target::DFlow => HttpResponse::NotImplemented().json(serde_json::json!({"error": "can't swap through the aggregators"})),
        Target::AMMs => {
            let (response_tx, response_rx) = oneshot::channel::<DispatchResponse>();
            let dispatch = DispatchParams::Swap { params: SwapParams::default(), response_tx };

            state.request_tx.send(dispatch).expect("send invalid transmitter req");
            tracing::info!("sent from `API Server::swap` towards `Strategy`");
            let response = response_rx.recv();
            tracing::info!("received from `Executor`");

            match response {
                Ok(response) => HttpResponse::Ok().json(response),
                Err(_) => HttpResponse::InternalServerError().json(json!({"error": "no response"})),
            }
        }
    }
}

/// implementation courtesy to https://docs.rs/solana-keypair/3.0.0/src/solana_keypair/lib.rs.html#154-189
/// Reads a JSON-encoded `Keypair` from a `Reader` implementor
pub fn read_keypair(reader: &str) -> eyre::Result<Keypair> {
    let trimmed = reader.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Input must be a JSON array").into());
    }

    // we already checked that the string has at least two chars,
    // so 1..trimmed.len() - 1 won't be out of bounds
    let contents = &trimmed[1..trimmed.len() - 1];
    let elements_vec: Vec<&str> = contents.split(',').map(|s| s.trim()).collect();
    let len = elements_vec.len();

    let elements: [&str; ed25519_dalek::KEYPAIR_LENGTH] =
        elements_vec.try_into().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Expected {} elements, found {}", ed25519_dalek::KEYPAIR_LENGTH, len)))?;
    let mut out = [0u8; ed25519_dalek::KEYPAIR_LENGTH];
    for (idx, element) in elements.into_iter().enumerate() {
        let parsed: u8 = element.parse()?;
        out[idx] = parsed;
    }

    Keypair::try_from(&out[..]).map_err(|e| std::io::Error::other(e.to_string()).into())
}
