use clap::{Parser, value_parser};
use secrecy::SecretString;

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = None
)]
pub struct Args {
    #[arg(long, env = "HTTP_URL", default_value = "https://api.mainnet-beta.solana.com")]
    pub http_url: SecretString,

    #[arg(long, env = "YELLOWSTONE_URL")]
    pub yellowstone_url: Option<SecretString>,

    #[arg(long, env = "YELLOWSTONE_X_TOKEN")]
    pub yellowstone_x_token: Option<SecretString>,

    #[arg(long, env = "BOOTSTRAP_FILE", default_value = "cfg/payloads/pmms.json")]
    pub bootstrap_file: String,

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
