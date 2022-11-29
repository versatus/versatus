use std::{fmt::Debug, net::SocketAddr, time::Duration};

use axum_server::tls_rustls::RustlsConfig;

/// Configuration store for an HttpApiServer
// TODO: implement a builder over this config.
// Source<: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
#[derive(Debug, Clone)]
pub struct HttpApiServerConfig {
    pub address: String,
    pub api_title: String,
    pub api_version: String,
    pub server_timeout: Option<Duration>,
    pub tls_config: Option<RustlsConfig>,
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
