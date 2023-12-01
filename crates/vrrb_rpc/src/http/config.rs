use std::{fmt::Debug, net::SocketAddr, time::Duration};

use axum_server::tls_rustls::RustlsConfig;

/// Configuration store for an HttpApiServer
// Source<: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html

#[derive(Debug)]
pub struct HttpApiServerConfig {
    pub(crate) address: String,
    pub(crate) api_title: String,
    pub(crate) api_version: String,
    pub(crate) server_timeout: Option<Duration>,
    pub(crate) tls_config: Option<RustlsConfig>,
}

impl From<HttpApiServerConfigBuilder> for HttpApiServerConfig {
    fn from(value: HttpApiServerConfigBuilder) -> HttpApiServerConfig {
        HttpApiServerConfig {
            address: value.address.expect("expected server address"),
            api_title: value.api_title.expect("expected server api title"),
            api_version: value.api_version.expect("expected server api version"),
            server_timeout: value.server_timeout,
            tls_config: value.tls_config,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct HttpApiServerConfigBuilder {
    address: Option<String>,
    api_title: Option<String>,
    api_version: Option<String>,
    server_timeout: Option<Duration>,
    tls_config: Option<RustlsConfig>,
}

impl HttpApiServerConfigBuilder {
    pub fn address(mut self, address: &str) -> Self {
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
    pub fn tls_config(mut self, tls_config: Option<RustlsConfig>) -> Self {
        self.tls_config = tls_config;
        self
    }
    pub fn build(self) -> HttpApiServerConfig {
        self.into()
    }
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
