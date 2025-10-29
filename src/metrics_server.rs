use actix_web::{App, HttpResponse, HttpServer, web};
use metrics_exporter_prometheus::PrometheusHandle;

#[derive(Debug, Clone)]
pub struct MetricsServerCfg {
    pub host: String,
    pub prometheus: PrometheusHandle,
}

pub struct MetricsServer {
    inner: actix_web::dev::Server,
}

impl MetricsServer {
    pub fn new(cfg: MetricsServerCfg) -> eyre::Result<MetricsServer> {
        let prometheus = cfg.prometheus.clone();

        Ok(MetricsServer {
            inner: HttpServer::new(move || {
                App::new()
                    // shared data across all endpoints/requests
                    .app_data(web::Data::new(prometheus.clone()))
                    // routes
                    .route("/metrics", web::get().to(metrics))
                    .route("/health", web::get().to(HttpResponse::Ok))
            })
            .bind(cfg.host.as_str())?
            .run(),
        })
    }

    pub async fn start(self) -> std::io::Result<()> {
        self.inner.await
    }
}

async fn metrics(handle: web::Data<PrometheusHandle>) -> HttpResponse {
    HttpResponse::Ok().content_type("text/plain").body(handle.render())
}
