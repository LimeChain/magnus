pub mod api_server;
pub mod args;
pub mod metrics_server;

use std::sync::mpsc;

use clap::Parser;
use futures_util::StreamExt as _;
use magnus::{
    EmptyCtx, Executor, Ingest, Strategy,
    adapters::{QuoteAndSwapResponse, QuoteParams, SwapAndAccountMetas},
    bootstrap::{Bootstrap, MarketRaw},
    executor::{BaseExecutor, BaseExecutorCfg},
    ingest::{GeyserPoolStateIngestor, IngestorCfg},
    solve::{DispatchParams, DispatchResponse, Solver, SolverCfg},
};
use metrics::describe_counter;
use metrics_exporter_prometheus::PrometheusBuilder;
use secrecy::ExposeSecret;
use solana_sdk::pubkey::Pubkey;
use tokio::signal::unix::{SignalKind, signal};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt::time::UtcTime};
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};

use crate::api_server::ApiServerCfg;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_thread_ids(true)
        .with_line_number(true)
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

// ingestor, solver, sendtx
async fn run_core_components() {}

// put all optional components behind feature flags
async fn run_components() {
    run_core_components().await;

    // api

    // metrics server
}

#[derive(Clone, Debug)]
pub struct AmmProgram {
    owner: Pubkey,
    markets: Vec<MarketRaw>,
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

    let client_http = std::sync::Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(cfg.http_url));
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

    let markets_raw = match &cfg.bootstrap_file {
        Some(file) => Bootstrap::ingest_from_file(file).expect("unable to ingest from file"),
        None => Bootstrap::ingest_from_jupiter().await.expect("unable to ingest from jupiter"),
    };

    let programs = Bootstrap::get_program_markets(&markets_raw);
    let markets = Bootstrap::init_markets(programs).await.expect("unable to initialise markets");
    let account_map = Bootstrap::acquire_account_map(&client_http, &markets).await.expect("unable to acquire account map");

    let program_markets = Bootstrap::get_program_markets(&markets_raw);
    info!("{:#?}", markets);

    /* prior spawning the ingestor, we'll need to ensure that the current state is actually fetched
     * through the geyser client
     */

    // ..
    let bare_ctx = EmptyCtx;

    /*
     * - The API server sends a signal to the solver once we need a quote and/or swap executed.
     * - The solver then proceeds to evaluate the best quote/swap based on the current market conditions (local state)
     *    and sends a message towards the executor thread, where we proceed to execute the swap.
     * - Once the swap is executed, the executor thread sends a message towards the API server.
     */
    /* sender == API server | receiver = Solver thread */
    let (request_tx, request_rx) = mpsc::channel::<DispatchParams>();
    /* sender = Solver thread | receiver = Executor thread */
    let (response_tx, response_rx) = mpsc::channel::<Vec<SwapAndAccountMetas>>();
    /* sender = Executor thread */
    let (executor_tx, _) = mpsc::channel::<DispatchResponse>();

    let _ = {
        let cfg = IngestorCfg { client_geyser, client_default: client_http.clone(), program_markets, markets: markets.clone(), account_map };
        tokio::spawn(async move { GeyserPoolStateIngestor::new(cfg).ingest(bare_ctx).await });
    };
    let _ = {
        let cfg = SolverCfg { markets, rx: request_rx, tx: response_tx };
        tokio::spawn(async move { Solver::new(cfg).compute(bare_ctx).await });
    };
    let _ = {
        let cfg = BaseExecutorCfg { client: client_http, solver_rx: response_rx, executor_tx };
        tokio::spawn(async move { BaseExecutor::new(cfg).execute(bare_ctx).await });
    };

    let server_handle = {
        let cfg = ApiServerCfg { host: cfg.api_server_host, workers: cfg.api_server_workers, request_tx };
        let server = api_server::ApiServer::new(cfg).expect("failed to create server");
        let handle = server.handle().clone();
        tokio::spawn(async move { server.start().await.expect("failed to start server") });

        handle
    };

    #[cfg(feature = "metrics")]
    let _ = tokio::spawn(async move {
        metrics_server::MetricsServer::new(metrics_server::MetricsServerCfg { host: cfg.metrics_server_host, workers: cfg.metrics_server_workers, prometheus })
            .expect("failed to create server")
            .start()
            .await
            .expect("failed to start server")
    });

    tokio::select! {
        _ = interrupt.recv() => {
            server_handle.stop(true).await;
        }
        _ = terminate.recv() => {
            server_handle.stop(false).await;
        }
    }
}

pub fn initialise_prometheus_metrics() {
    describe_counter!("API HITS", "The amount of hits experienced by the API since the server started");
    describe_counter!("API ERRORS", "The amount of errors experienced by the API since the server started");

    describe_counter!("METRICS HITS", "The amount of hits experienced by /metrics since the (metrics) server started");
    // ..
}
