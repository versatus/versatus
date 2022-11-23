use std::{convert::Infallible, fmt::Debug, io, time::Duration};

use poem::{
    listener::{Acceptor, TcpAcceptor},
    Route, Server,
};
use poem_openapi::OpenApiService;
use tokio::sync::broadcast::Receiver;
use vrrb_core::event_router::Event;

use crate::http::HttpApi;
use crate::http::HttpApiRouterConfig;
use crate::http::HttpApiServerConfig;

/// A JSON-RPC API layer for VRRB nodes.
pub struct HttpApiServer {
    server: Server<Infallible, TcpAcceptor>,
    server_timeout: Option<Duration>,
    app: Route,
}

impl Debug for HttpApiServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpApiServer")
            .field("server_timeout", &self.server_timeout)
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

        let mut app = Self::create_router(&router_config)?;
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
    pub async fn start(self, ctrl_rx: &mut Receiver<Event>) -> io::Result<()> {
        let server_timeout = self.server_timeout;
        let server = self.server;
        let app = self.app;

        server
            .run_with_graceful_shutdown(
                app,
                async {
                    if let Err(err) = ctrl_rx.recv().await {
                        telemetry::info!("Failed to process shutdown signal. Reason: {err}");
                    }
                },
                server_timeout,
            )
            .await
    }

    pub fn create_router(config: &HttpApiRouterConfig) -> Result<Route, String> {
        let openapi_service = OpenApiService::new(
            HttpApi,
            config.api_title.clone(),
            config.api_version.clone(),
        )
        .server(config.address.to_string());

        let ui = openapi_service.swagger_ui();
        let router = Route::new().nest("/", openapi_service).nest("/docs", ui);

        Ok(router)
    }
}
