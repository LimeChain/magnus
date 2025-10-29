pub mod args;
pub mod metrics_server;

use clap::Parser;
use metrics_exporter_prometheus::PrometheusBuilder;
use secrecy::ExposeSecret;
use tokio::signal::unix::{SignalKind, signal};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt::time::UtcTime};

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
        metrics_server_host: args.metrics_server_host,
        api_server_host: args.api_server_host,
    };

    run(cfg).await;
}

pub struct Cfg {
    http_url: String,
    ws_url: String,
    metrics_server_host: String,
    api_server_host: String,
}

async fn run(cfg: Cfg) {
    let mut interrupt = signal(SignalKind::interrupt()).expect("Unable to initialise interrupt signal handler");
    let mut terminate = signal(SignalKind::terminate()).expect("Unable to initialise termination signal handler");

    let prometheus = PrometheusBuilder::new().install_recorder().expect("failed to install recorder");

    /*
     * |1| Bootstrap proper state, either natively or via some external provider (check out `https://cache.jup.ag/markets?v=4`)
     * |2| Define concrete, uniform interfaces for all the adapters
     * |3| Implement a proper routing engine based on some defined constraints
     */

    let _ = tokio::spawn(async {
        metrics_server::MetricsServer::new(metrics_server::MetricsServerCfg { host: cfg.metrics_server_host, prometheus })
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

async fn state() {}
