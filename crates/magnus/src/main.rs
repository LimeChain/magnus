pub mod args;

use std::sync::mpsc;

use clap::Parser;
#[cfg(feature = "metrics")]
use magnus::metrics_server;
use magnus::{
    EmptyCtx, Executor, Ingest, Strategy,
    api_server::{self, ApiServerCfg},
    bootstrap,
    executor::{BaseExecutor, BaseExecutorCfg},
    ingest::{GeyserPoolStateIngestor, IngestorCfg},
    strategy::{BaseStrategy, BaseStrategyCfg, DispatchParams, WrappedSwapAndAccountMetas},
};
use secrecy::ExposeSecret;
use tokio::signal::unix::{SignalKind, signal};
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, fmt::time::UtcTime};
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};

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
    yellowstone_url: Option<String>,
    yellowstone_x_token: Option<String>,
    bootstrap_file: String,
    api_server_host: String,
    api_server_workers: u16,
    metrics_server_host: String,
    metrics_server_workers: u16,
}

async fn run(cfg: Cfg) {
    let mut interrupt = signal(SignalKind::interrupt()).expect("Unable to initialise interrupt signal handler");
    let mut terminate = signal(SignalKind::terminate()).expect("Unable to initialise termination signal handler");

    #[cfg(feature = "metrics")]
    let prometheus = metrics_exporter_prometheus::PrometheusBuilder::new().install_recorder().expect("failed to install recorder");
    #[cfg(feature = "metrics")]
    metrics_server::initialise_prometheus_description_metrics();

    let client_http = std::sync::Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(cfg.http_url));
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

    let pmms = bootstrap::load(&cfg.bootstrap_file).expect("unable to load bootstrap file");
    let markets = bootstrap::into_markets(pmms);
    let account_map = bootstrap::acquire_account_map(&client_http, &markets).await.expect("unable to acquire account map");
    debug!(?account_map);

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
    let (response_tx, response_rx) = mpsc::channel::<WrappedSwapAndAccountMetas>();

    {
        let cfg = IngestorCfg { client_geyser, client_default: client_http.clone(), markets: markets.clone(), account_map };
        tokio::spawn(async move { GeyserPoolStateIngestor::new(cfg).ingest(bare_ctx).await });
    };

    {
        let cfg = BaseStrategyCfg { markets, api_server_rx: request_rx, tx: response_tx };
        tokio::spawn(async move { BaseStrategy::new(cfg).compute(bare_ctx).await });
    };

    {
        let cfg = BaseExecutorCfg { client: client_http, solver_rx: response_rx };
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
    {
        let cfg = metrics_server::MetricsServerCfg { host: cfg.metrics_server_host, workers: cfg.metrics_server_workers, prometheus };
        tokio::spawn(async move { metrics_server::MetricsServer::new(cfg).expect("failed to create server").start().await.expect("failed to start server") });
    }

    tokio::select! {
        _ = interrupt.recv() => {
            server_handle.stop(true).await;
        }
        _ = terminate.recv() => {
            server_handle.stop(false).await;
        }
    }
}
