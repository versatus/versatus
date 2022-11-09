use std::{convert::Infallible, fmt::Debug, io, net::SocketAddr, time::Duration};

use poem::{
    listener::{Acceptor, TcpAcceptor},
    Route,
    Server,
};
use poem_openapi::{payload::PlainText, OpenApi, OpenApiService};
use tokio::sync::mpsc::Receiver;

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
#[derive(Debug, Clone)]
pub struct HttpApiServerConfig {
    pub address: String,
    pub api_title: String,
    pub api_version: String,
    pub server_timeout: Option<Duration>,
}

/// Configuration store for an HttpApiRouter
// TODO: implement a builder over this config.
// Source<: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
#[derive(Debug, Clone)]
pub struct HttpApiRouterConfig {
    pub address: SocketAddr,
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

impl Debug for HttpApiServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpApiServer")
            // .field("server", &self.server)
            .field("server_timeout", &self.server_timeout)
            // .field("app", &self.app)
            .finish()
    }
}

impl HttpApiServer {
    // TODO: refactor return type into a proper crate specific result
    pub fn new(config: HttpApiServerConfig) -> Result<Self, String> {
        let listener = std::net::TcpListener::bind(config.address.clone())
            .map_err(|_err| format!("unable to bind to address: {}", config.address))?;

        let acceptor = TcpAcceptor::from_std(listener)
            .map_err(|_err| format!("unable to bind to listener on address: {}", config.address))?;

        let address = acceptor.local_addr();
        let address = address
            .get(0)
            .ok_or_else(|| String::from("unable to retrieve the address the server is bound to"))?;

        let address = address
            .as_socket_addr()
            .ok_or_else(|| String::from("unable to retrieve the address the server is bound to"))?;

        let router_config = HttpApiRouterConfig {
            address: *address,
            api_title: config.api_title.clone(),
            api_version: config.api_version.clone(),
            server_timeout: config.server_timeout,
        };

        let app = Self::create_router(&router_config)?;
        let server = Server::new_with_acceptor(acceptor);
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

    fn create_router(config: &HttpApiRouterConfig) -> Result<Route, String> {
        let openapi_service = OpenApiService::new(
            HttpApi,
            config.api_title.clone(),
            config.api_version.clone(),
        )
        .server(config.address.to_string());

        let ui = openapi_service.swagger_ui();

        Ok(Route::new().nest("/", openapi_service).nest("/docs", ui))
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use poem::{
        listener::{Acceptor, TcpAcceptor},
        test::TestClient,
    };

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
            address: addr.as_socket_addr().unwrap().clone(),
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
