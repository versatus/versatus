use std::fmt::Debug;
use std::net::{SocketAddr, TcpListener, TcpStream};

use axum_server::tls_rustls::RustlsConfig;
use tokio::sync::broadcast::Receiver;
use vrrb_core::event_router::Event;

use crate::http::router::create_router;
use crate::http::HttpApiRouterConfig;
use crate::http::HttpApiServerConfig;
use crate::{ApiError, Result};
use axum::{Router, Server};

/// A JSON-RPC API layer for VRRB nodes.
pub struct HttpApiServer {
    router: Router,
    listener: TcpListener,
    tls_config: Option<RustlsConfig>,
}

impl Debug for HttpApiServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpApiServer")
            .field("server_timeout", &self.listener.local_addr())
            .finish()
    }
}

impl HttpApiServer {
    pub fn new(config: HttpApiServerConfig) -> Result<Self> {
        let address = &config.address.parse().unwrap();

        let router_config = HttpApiRouterConfig {
            address: *address,
            api_title: config.api_title.clone(),
            api_version: config.api_version.clone(),
            server_timeout: config.server_timeout,
        };

        let tls_config = config.tls_config;
        let router = create_router(&router_config);
        let listener = TcpListener::bind(address).map_err(|err| {
            ApiError::Other(format!("unable to bind to address {address}: {}", err))
        })?;

        Ok(Self {
            router,
            listener,
            tls_config,
        })
    }

    pub fn address(&self) -> Result<SocketAddr> {
        self.listener.local_addr().map_err(|err| {
            ApiError::Other(format!(
                "unable to retrieve the server's local address. Reason: {}",
                err
            ))
        })
    }

    /// Starts listening for HTTP connections on the configured address.
    /// NOTE: this method needs to consume the instance of HttpApiServer
    pub async fn start(self, ctrl_rx: &mut Receiver<Event>) -> Result<()> {
        let addr = self.address()?;

        dbg!(&addr, &self.tls_config);

        if let Some(tls_config) = self.tls_config {
            let tls_server = axum_server::from_tcp_rustls(self.listener, tls_config)
                .serve(self.router.into_make_service());

            if let Err(err) = tls_server.await {
                telemetry::error!("server error: {err}");
                return Err(ApiError::Other(err.to_string()));
            }

            dbg!("made it here");

            return Ok(());
        }

        let server = Server::from_tcp(self.listener)
            .map_err(|err| ApiError::Other(format!("unable to bind to listener: {err}")))?;

        let graceful = server
            .serve(self.router.into_make_service())
            .with_graceful_shutdown(async {
                if let Err(err) = ctrl_rx.recv().await {
                    telemetry::error!("failed to listen for shutdown signal: {err}");
                }
                telemetry::info!("shutting down server");
            });

        if let Err(err) = graceful.await {
            telemetry::error!("server error: {err}");
            return Err(ApiError::Other(err.to_string()));
        }

        Ok(())
    }
}
