use tracing_subscriber::{EnvFilter, fmt::time::UtcTime};

#[tokio::main]
pub async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_thread_ids(true)
        .with_target(true)
        .with_timer(UtcTime::rfc_3339())
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::default().add_directive(tracing::Level::INFO.into())))
        .init();

    let _client_http = solana_client::rpc_client::RpcClient::new(std::env::var("RPC_URL").unwrap());
    let _client_ws = solana_client::nonblocking::pubsub_client::PubsubClient::new(&std::env::var("WS_URL").unwrap()).await.expect("unable to create websocket client");

    Ok(())
}
