use clap::{Parser, ValueEnum, value_parser};
use secrecy::SecretString;

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = None
)]
pub struct Args {
    #[arg(long, env = "HTTP_URL", default_value = "http://127.0.0.1:8899")]
    pub http_url: SecretString,

    #[arg(long, env = "WS_URL", default_value = "ws://127.0.0.1:8900")]
    pub ws_url: SecretString,

    #[arg(long, env = "YELLOWSTONE_URL")]
    pub yellowstone_url: Option<SecretString>,

    #[arg(long, env = "YELLOWSTONE_X_TOKEN")]
    pub yellowstone_x_token: Option<SecretString>,

    #[arg(long, env = "BOOTSTRAP_FILE")]
    pub bootstrap_file: Option<String>,

    #[arg(long, env = "API_SERVER_HOST", default_value = "0.0.0.0:19000")]
    pub api_server_host: String,

    #[arg(
        long,
        env = "API_SERVER_WORKERS",
        default_value = "4",
        value_parser = value_parser!(u16).range(1..)
    )]
    pub api_server_workers: u16,

    #[arg(long, env = "METRICS_SERVER_HOST", default_value = "0.0.0.0:19001")]
    pub metrics_server_host: String,

    #[arg(
        long,
        env = "METRICS_SERVER_WORKERS",
        default_value = "2",
        value_parser = value_parser!(u16).range(1..)
    )]
    pub metrics_server_workers: u16,
}
