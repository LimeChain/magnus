use std::{net::TcpListener, sync::mpsc};

use magnus::{
    api_server::{ApiServer, ApiServerCfg},
    strategy::DispatchParams,
};

pub struct TestServer {
    pub base_url: String,
    pub request_rx: mpsc::Receiver<DispatchParams>,
    pub server_handle: actix_web::dev::ServerHandle,
}

impl TestServer {
    /// Spawns a new API server on a random available port for testing
    pub async fn spawn() -> Self {
        // Find an available port
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
        let port = listener.local_addr().unwrap().port();
        drop(listener); // Release the port for the server to use

        let host = format!("127.0.0.1:{}", port);
        let base_url = format!("http://{}", host);

        let (request_tx, request_rx) = mpsc::channel::<DispatchParams>();

        let cfg = ApiServerCfg {
            host: host.clone(),
            workers: 1, // Use single worker for tests
            request_tx,
        };

        let server = ApiServer::new(cfg).expect("Failed to create test server");
        let server_handle = server.handle().clone();

        tokio::spawn(async move {
            server.start().await.expect("Server failed to start");
        });

        let client = reqwest::Client::new();
        let mut retries = 0;
        loop {
            if retries > 50 {
                panic!("Server failed to start within timeout");
            }

            match client.get(&format!("{}/health", base_url)).send().await {
                Ok(_) => break,
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    retries += 1;
                }
            }
        }

        TestServer { base_url, request_rx, server_handle }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

#[tokio::test]
async fn test_health_endpoint() {
    let server = TestServer::spawn().await;
    let client = reqwest::Client::new();

    let response = client.get(&server.url("/health")).send().await.expect("Failed to send request");

    assert_eq!(response.status(), 200);
}
