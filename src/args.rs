use clap::Parser;
use secrecy::SecretString;

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = None
)]
pub struct Args {
    #[arg(long, env = "HTTP_URL", default_value = "http://localhost:8545")]
    pub http_url: SecretString,

    #[arg(long, env = "WS_URL", default_value = "ws://localhost:8545")]
    pub ws_url: SecretString,

    #[arg(long, env = "METRICS_SERVER_HOST", default_value = "0.0.0.0:19000")]
    pub metrics_server_host: String,

    #[arg(long, env = "API_SERVER_HOST", default_value = "0.0.0.0:19001")]
    pub api_server_host: String,
}
