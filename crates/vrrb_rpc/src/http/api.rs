use poem_openapi::{payload::PlainText, OpenApi};
use std::fmt::Debug;

use crate::http::config::*;

#[derive(Debug, Clone)]
pub struct HttpApi;

#[OpenApi]
impl HttpApi {
    /// Health check route
    #[oai(path = "/health", method = "get")]
    #[telemetry::instrument(name = "healthcheck")]
    async fn index(&self) -> PlainText<&'static str> {
        PlainText("ok")
    }

    /// Health check route
    #[oai(path = "/accounts", method = "get")]
    #[telemetry::instrument(name = "get_accounts")]
    async fn accounts(&self) -> PlainText<&'static str> {
        PlainText("accounts")
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use poem::{
        listener::{Acceptor, TcpAcceptor},
        test::TestClient,
    };

    use crate::http::HttpApiServer;

    use super::*;

    #[tokio::test]
    async fn index_returns_openapi_docs() {
        let listener = std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        let acceptor = TcpAcceptor::from_std(listener).unwrap();
        let addr = acceptor.local_addr();
        let addr = addr.get(0).unwrap();

        let _api_title = "Node HTTP API".to_string();
        let _api_version = "1.0".to_string();

        let config = HttpApiRouterConfig {
            address: *addr.as_socket_addr().unwrap(),
            api_title: "Node HTTP API".into(),
            api_version: "1.0".into(),
            server_timeout: None,
        };

        let app = HttpApiServer::create_router(&config).unwrap();
        let resp = TestClient::new(app).get("/health").send().await;
        resp.assert_status_is_ok();
        resp.assert_text("ok").await;
    }
}
