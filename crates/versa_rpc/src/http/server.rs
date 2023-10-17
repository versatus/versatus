use std::{
    fmt::Debug,
    net::{SocketAddr, TcpListener},
    time::Duration,
};

use axum::{Router, Server};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use events::Event;
use tokio::sync::broadcast::Receiver;

use crate::{
    http::{router::create_router, HttpApiRouterConfig, HttpApiServerConfig},
    ApiError,
    Result,
};

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
            ApiError::Other(format!("unable to bind to address {address}: {err}"))
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
                "unable to retrieve the server's local address. Reason: {err}"
            ))
        })
    }

    /// Starts listening for HTTP connections on the configured address.
    /// NOTE: this method needs to consume the instance of HttpApiServer
    pub async fn start(self, ctrl_rx: &mut Receiver<Event>) -> Result<()> {
        if let Some(tls_config) = self.tls_config {
            let handle = Handle::new();

            let tls_server = axum_server::from_tcp_rustls(self.listener, tls_config)
                .handle(handle.clone())
                .serve(self.router.into_make_service());

            let server_handle = tokio::spawn(async move {
                if let Err(err) = tls_server.await {
                    telemetry::error!("server error: {err}");
                    return Err(ApiError::Other(err.to_string()));
                }

                Ok(())
            });

            if let Err(err) = ctrl_rx.recv().await {
                telemetry::error!("failed to listen for shutdown signal: {err}");
            }

            telemetry::info!("shutting down server");

            handle.graceful_shutdown(Some(Duration::from_secs(30)));

            return server_handle
                .await
                .map_err(|err| ApiError::Other(format!("failed to join server handle: {err}")))?;
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
