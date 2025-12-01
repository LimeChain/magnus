pub mod api_server;
pub mod args;
pub mod metrics_server;

use clap::Parser;
use futures_util::StreamExt as _;
use magnus::{
    bootstrap::Bootstrap,
    geyser_client::GeyserClientWrapped,
    helpers::{deserialize_anchor_account, geyser_acc_to_native},
    ingest::GeyserPoolStateIngestor,
};
use metrics::describe_counter;
use metrics_exporter_prometheus::PrometheusBuilder;
use secrecy::ExposeSecret;
use solana_sdk::pubkey::Pubkey;
use tokio::signal::unix::{SignalKind, signal};
use tracing::{debug, error, info};
use tracing_subscriber::{EnvFilter, fmt::time::UtcTime};
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::geyser::subscribe_update;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_thread_ids(true)
        .with_target(true)
        .with_timer(UtcTime::rfc_3339())
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::default().add_directive(tracing::Level::INFO.into())))
        .init();

    let args = args::Args::parse();
    info!(?args);

    let cfg = Cfg {
        http_url: args.http_url.expose_secret().into(),
        ws_url: args.ws_url.expose_secret().into(),
        yellowstone_url: args.yellowstone_url.map(|v| v.expose_secret().into()),
        yellowstone_x_token: args.yellowstone_x_token.map(|v| v.expose_secret().into()),
        bootstrap_file: args.bootstrap_file,
        api_server_host: args.api_server_host,
        api_server_workers: args.api_server_workers,
        metrics_server_host: args.metrics_server_host,
        metrics_server_workers: args.metrics_server_workers,
    };

    run(cfg).await;
}

pub struct Cfg {
    http_url: String,
    ws_url: String,
    yellowstone_url: Option<String>,
    yellowstone_x_token: Option<String>,
    bootstrap_file: Option<String>,
    api_server_host: String,
    api_server_workers: u16,
    metrics_server_host: String,
    metrics_server_workers: u16,
}

async fn run(cfg: Cfg) {
    let mut interrupt = signal(SignalKind::interrupt()).expect("Unable to initialise interrupt signal handler");
    let mut terminate = signal(SignalKind::terminate()).expect("Unable to initialise termination signal handler");

    let prometheus = PrometheusBuilder::new().install_recorder().expect("failed to install recorder");
    initialise_prometheus_metrics();

    /*
     * |1| Bootstrap proper state, either natively or via some external provider (check out `https://cache.jup.ag/markets?v=4`)
     * |2| Define concrete, uniform interfaces for all the adapters
     * |3| Implement a proper routing engine based on some defined constraints
     */

    let client_http = solana_client::rpc_client::RpcClient::new(cfg.http_url);
    let client_ws = solana_client::nonblocking::pubsub_client::PubsubClient::new(&cfg.ws_url).await.expect("unable to create websocket client");
    let client_geyser = GeyserGrpcClient::build_from_shared(cfg.yellowstone_url.unwrap_or_default())
        .expect("invalid grpc url")
        .tls_config(ClientTlsConfig::new().with_native_roots())
        .expect("unable to craft a tls config")
        .x_token(cfg.yellowstone_x_token)
        .expect("unable to determine yellowstone x-token")
        .max_decoding_message_size(1024 * 1024 * 1024)
        .connect()
        .await
        .expect("unable to connect");

    let markets = match &cfg.bootstrap_file {
        Some(file) => Bootstrap::ingest_from_file(file).expect("unable to ingest from file"),
        None => Bootstrap::ingest_from_jupiter().await.expect("unable to ingest from jupiter"),
    };
    let accounts = markets.iter().map(|market| market.pubkey.to_string()).collect::<Vec<_>>();
    debug!("loaded accounts | {:?}", accounts);

    /* prior spawning the ingestor, we'll need to ensure that the current state is actually fetched
     * through the geyser client
     */
    let _ = tokio::spawn(async move { GeyserPoolStateIngestor::new(client_geyser, accounts).ingest().await });

    tokio::spawn(async move {
        api_server::ApiServer::new(api_server::ApiServerCfg { host: cfg.api_server_host, workers: cfg.api_server_workers })
            .expect("failed to create server")
            .start()
            .await
            .expect("failed to start server")
    });

    tokio::spawn(async move {
        metrics_server::MetricsServer::new(metrics_server::MetricsServerCfg { host: cfg.metrics_server_host, workers: cfg.metrics_server_workers, prometheus })
            .expect("failed to create server")
            .start()
            .await
            .expect("failed to start server")
    });

    tokio::select! {
        _ = interrupt.recv() => {}
        _ = terminate.recv() => {}
    }
}

pub fn initialise_prometheus_metrics() {
    describe_counter!("API HITS", "The amount of hits experienced by the API since the server started");
    describe_counter!("API ERRORS", "The amount of errors experienced by the API since the server started");

    describe_counter!("METRICS HITS", "The amount of hits experienced by the /metrics since the (metrics) server started");
    // ..
}
