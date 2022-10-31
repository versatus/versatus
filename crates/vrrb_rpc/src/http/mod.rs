use std::{convert::Infallible, io, time::Duration};

use poem::{
    handler,
    listener::{Acceptor, TcpAcceptor, TcpListener},
    web::{Json, LocalAddr},
    Endpoint, Route, Server,
};
use poem_openapi::{payload::PlainText, OpenApi, OpenApiService};
use tokio::sync::mpsc::{Receiver, UnboundedReceiver};
use vrrb_core::event_router::Event;

struct HttpApi;

#[OpenApi]
impl HttpApi {
    /// Healthcheck route
    #[oai(path = "/health", method = "get")]
    async fn index(&self) -> PlainText<&'static str> {
        PlainText("ok")
    }
}

/// Configuration store for an HttpApiServer
// TODO: implement a builder over this config.
// Source<: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
pub struct HttpApiServerConfig {
    pub acceptor: TcpAcceptor,
    pub api_title: String,
    pub api_version: String,
    pub server_timeout: Option<Duration>,
}

/// A JSON-RPC API layer for VRRB nodes.
pub struct HttpApiServer {
    server: Server<Infallible, TcpAcceptor>,
    server_timeout: Option<Duration>,
    app: Route,
}

impl HttpApiServer {
    // TODO: refactor return type into a proper crate specific result
    pub fn new(config: HttpApiServerConfig) -> Result<Self, String> {
        let app = Self::create_router(&config)?;
        let server = Server::new_with_acceptor(config.acceptor);
        let server_timeout = config.server_timeout;

        Ok(Self {
            server,
            server_timeout,
            app,
        })
    }

    /// Starts listening for HTTP connections on the configured address.
    /// NOTE: this method needs to consume the instance of HttpApiServer
    pub async fn start(self, ctrl_rx: &mut Receiver<()>) -> io::Result<()> {
        let server_timeout = self.server_timeout;
        let server = self.server;
        let app = self.app;

        server
            .run_with_graceful_shutdown(
                app,
                async {
                    ctrl_rx.recv().await;
                },
                server_timeout,
            )
            .await
    }

    fn create_router(config: &HttpApiServerConfig) -> Result<Route, String> {
        let address = config.acceptor.local_addr();

        let address = address
            .get(0)
            .ok_or_else(|| String::from("Unable to bind to provided address"))?;

        let openapi_service = OpenApiService::new(
            HttpApi,
            config.api_title.clone(),
            config.api_version.clone(),
        )
        .server(address.to_string());

        let ui = openapi_service.swagger_ui();

        Ok(Route::new().nest("/", openapi_service).nest("/docs", ui))
    }
}

#[cfg(test)]
mod tests {
    use poem::{
        get,
        listener::{Acceptor, TcpAcceptor},
        Endpoint,
    };

    use tokio::{signal::unix::SignalKind, sync::mpsc::channel};

    use super::*;
    use poem::test::TestClient;
    use poem::Route;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[tokio::test]
    async fn index_returns_openapi_docs() {
        let listener = std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        let acceptor = TcpAcceptor::from_std(listener).unwrap();
        let addr = acceptor.local_addr();
        let addr = addr.get(0).unwrap();

        let api_title = "Node HTTP API".to_string();
        let api_version = "1.0".to_string();

        let config = HttpApiServerConfig {
            acceptor,
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
