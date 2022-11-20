use std::{convert::Infallible, fmt::Debug, io, net::SocketAddr, time::Duration};

use poem::{
    listener::{Acceptor, TcpAcceptor},
    middleware::AddData,
    EndpointExt, Route, Server,
};
use poem_openapi::{payload::PlainText, OpenApi, OpenApiService};
use tokio::sync::mpsc::Receiver;

mod api;
mod config;
mod server;

pub use api::*;
pub use config::*;
pub use server::*;
