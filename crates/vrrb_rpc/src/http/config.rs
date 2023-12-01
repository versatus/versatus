use std::{fmt::Debug, net::SocketAddr, time::Duration};

use axum_server::tls_rustls::RustlsConfig;

/// Configuration store for an HttpApiServer
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
// Source<: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
#[derive(Debug, Clone)]
pub struct HttpApiRouterConfig {
    pub address: SocketAddr,
    pub api_title: String,
    pub api_version: String,
    pub server_timeout: Option<Duration>,
}

impl From<HttpApiRouterConfigBuilder> for HttpApiRouterConfig {
    fn from(value: HttpApiRouterConfigBuilder) -> HttpApiRouterConfig {
        HttpApiRouterConfig {
            address: value.address.expect("expected socket address"),
            api_title: value.api_title.expect("expected router api title"),
            api_version: value.api_version.expect("expected router api version"),
            server_timeout: value.server_timeout,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct HttpApiRouterConfigBuilder {
    address: Option<SocketAddr>,
    api_title: Option<String>,
    api_version: Option<String>,
    server_timeout: Option<Duration>,
}

impl HttpApiRouterConfigBuilder {
    pub fn address(mut self, address: SocketAddr) -> Self {
        self.address = Some(address.into());
        self
    }
    pub fn api_title(mut self, api_title: &str) -> Self {
        self.api_title = Some(api_title.into());
        self
    }
    pub fn api_version(mut self, api_version: &str) -> Self {
        self.api_version = Some(api_version.into());
        self
    }
    pub fn server_timeout(mut self, server_timeout: Option<Duration>) -> Self {
        self.server_timeout = server_timeout;
        self
    }
    pub fn build(self) -> HttpApiRouterConfig {
        self.into()
    }
}
