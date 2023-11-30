use std::{fmt::Debug, net::SocketAddr, time::Duration};

use axum_server::tls_rustls::RustlsConfig;

/// Configuration store for an HttpApiServer
// Source<: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html

#[derive(Debug)]
struct HttpApiServer {
    address: String,
    api_title: String,
    api_version: String,
    server_timeout: Duration,
    tls_config: RustlsConfig,
}

impl From<HttpApiServerConfigBuilder> for HttpApiServer {
    fn from(value: HttpApiServerConfigBuilder) -> HttpApiServer {
        HttpApiServer {
            address: value.address.expect("expected server address"),
            api_title: value.api_title.expect("expected server api title"),
            api_version: value.api_version.expect("expected server api version"),
            server_timeout: value
                .server_timeout
                .expect("expected server timeout duration"),
            tls_config: value.tls_config.expect("expected tls configuration"),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct HttpApiServerConfigBuilder {
    pub address: Option<String>,
    pub api_title: Option<String>,
    pub api_version: Option<String>,
    pub server_timeout: Option<Duration>,
    pub tls_config: Option<RustlsConfig>,
}

impl HttpApiServerConfigBuilder {
    fn address(mut self, address: &str) -> Self {
        self.address = Some(address.into());
        self
    }
    fn api_title(mut self, api_title: &str) -> Self {
        self.api_title = Some(api_title.into());
        self
    }
    fn api_version(mut self, api_version: &str) -> Self {
        self.api_version = Some(api_version.into());
        self
    }
    fn server_timeout(mut self, server_timeout: Duration) -> Self {
        self.server_timeout = Some(server_timeout);
        self
    }
    fn tls_config(mut self, tls_config: RustlsConfig) -> Self {
        self.tls_config = Some(tls_config);
        self
    }
    fn build(self) -> HttpApiServer {
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
