pub mod v1;

use std::sync::mpsc;

use actix_web::{App, HttpResponse, HttpServer, dev::ServerHandle, middleware::Logger, web};
#[cfg(feature = "metrics")]
use metrics::counter;
use tracing_actix_web::TracingLogger;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    api_server::v1::{quote, swap},
    strategy::DispatchParams,
};

#[derive(Debug)]
pub struct ApiServerCfg {
    pub host: String,
    pub workers: u16,
    pub request_tx: mpsc::Sender<DispatchParams>,
}

pub struct ApiServer {
    inner: actix_web::dev::Server,
    handle: actix_web::dev::ServerHandle,
}

#[derive(Clone)]
pub struct ServerState {
    pub request_tx: mpsc::Sender<DispatchParams>,
}

impl ApiServer {
    pub fn new(cfg: ApiServerCfg) -> eyre::Result<ApiServer> {
        #[derive(Copy, Clone, OpenApi)]
        #[openapi(paths(quote::quote_handler, swap::swap_handler))]
        struct ApiDoc;
        let openapi = ApiDoc::openapi();

        let state = ServerState { request_tx: cfg.request_tx.clone() };

        let http_server = HttpServer::new(move || {
            App::new()
                // state
                .app_data(web::Data::new(state.clone()))

                // middlewares
                .wrap(Logger::default())
                .wrap(TracingLogger::default())

                // routes - docs
                .service(RapiDoc::with_openapi("docs/openapi.json", openapi.clone()).path("/docs"))
                .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/swagger-ui/openapi.json", openapi.clone()))

                // routes - api
                .route("/health", web::get().to(health_handler))
                .service(
                    web::scope("/api").service(
                        web::scope("/v1")
                            // core ops
                            .route("/quote", web::get().to(quote::quote_handler))
                            .route("/swap", web::get().to(swap::swap_handler))

                            // miscellaneous
                            .route("/markets/supported", web::get().to(|| async { HttpResponse::NotImplemented().finish() })) // analytics?
                            .route("/markets/hotload", web::get().to(|| async { HttpResponse::NotImplemented().finish() })) // hotload new markets?
                        )
                )
        })
        .workers(cfg.workers as usize)
        .bind(cfg.host.as_str())?
        .disable_signals()
        .run();

        let handle = http_server.handle();

        Ok(ApiServer { inner: http_server, handle })
    }

    pub fn handle(&self) -> &ServerHandle {
        &self.handle
    }

    pub async fn start(self) -> std::io::Result<()> {
        self.inner.await
    }
}

pub async fn health_handler() -> HttpResponse {
    #[cfg(feature = "metrics")]
    counter!("API HITS", "health" => "/health").increment(1);

    HttpResponse::Ok().finish()
}
