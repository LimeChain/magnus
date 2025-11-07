use actix_web::{App, HttpResponse, HttpServer, web};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct ApiServerCfg {
    pub host: String,
    pub workers: u16,
}

pub struct ApiServer {
    inner: actix_web::dev::Server,
}

impl ApiServer {
    pub fn new(cfg: ApiServerCfg) -> eyre::Result<ApiServer> {
        Ok(ApiServer {
            inner: HttpServer::new(move || {
                App::new()
                    // routes
                    .route("/health", web::get().to(HttpResponse::Ok))
                    .service(
                        web::scope("/v1")
                            .route("/quote", web::post().to(|| async { HttpResponse::NotImplemented() }))
                            .route("/simulate", web::post().to(|| async { HttpResponse::NotImplemented() }))
                    )
            })
            .workers(cfg.workers as usize)
            .bind(cfg.host.as_str())?
            .run(),
        })
    }

    pub async fn start(self) -> std::io::Result<()> {
        self.inner.await
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteOrSimParam {
    input_mint: Pubkey,
    output_mint: Pubkey,
    amount: u64,
}
