use std::{convert::Infallible, fmt::Debug, io, net::SocketAddr, time::Duration};

use poem::{
    listener::{Acceptor, TcpAcceptor},
    middleware::AddData,
    EndpointExt, Route, Server,
};
use poem_openapi::{payload::PlainText, OpenApi, OpenApiService};
use tokio::sync::mpsc::Receiver;

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
