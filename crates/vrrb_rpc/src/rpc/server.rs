use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use async_trait::async_trait;
use axum_server::tls_rustls::RustlsConfig;
use hyper::{
    client::HttpConnector,
    server::{conn::Http, Server},
    service::{make_service_fn, service_fn},
};
// use tokio_rustls::TlsAcceptor;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use jsonrpsee::{
    core::Error,
    server::{ServerBuilder, ServerHandle, SubscriptionSink},
    types::SubscriptionResult,
};
use mempool::{LeftRightMempool, Mempool, MempoolReadHandleFactory};
use primitives::NodeType;
// use axum_server::{tls_rustls::RustlsConfig, Server};
use rustls::ClientConfig;
use storage::vrrbdb::{VrrbDb, VrrbDbConfig, VrrbDbReadHandle};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_rustls::{rustls, rustls::ServerConfig, TlsAcceptor};
use vrrb_core::{
    account::Account,
    event_router::{DirectedEvent, Event, Topic},
    txn::NewTxnArgs,
};

use super::tls;
use crate::rpc::{api::RpcApiServer, server_impl::RpcServerImpl};

#[derive(Debug, Clone)]
pub struct JsonRpcServerConfig {
    pub address: SocketAddr,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub node_type: NodeType,
    pub events_tx: UnboundedSender<DirectedEvent>,
    // pub tls_config: Option<ServerConfig>,
}

#[derive(Debug)]
pub struct JsonRpcServer;

impl JsonRpcServer {
    pub async fn run(config: &JsonRpcServerConfig) -> anyhow::Result<(ServerHandle, SocketAddr)> {
        let server = ServerBuilder::default().build(config.address).await?;

        let server_impl = RpcServerImpl {
            node_type: config.node_type,
            events_tx: config.events_tx.clone(),
            vrrbdb_read_handle: config.vrrbdb_read_handle.clone(),
            mempool_read_handle_factory: config.mempool_read_handle_factory.clone(),
        };

        // if let Some(tls_config) = &config.tls_config {
        //     // if let Err(e) = tls::run_server() {
        //     //     eprintln!("FAILED: {}", e);
        //     //     std::process::exit(1);
        //     // }

        //     tls::run_server();
        // }
        tokio::spawn(async move {
            // match create_tx_indexer(&txn_record).await {
            //     Ok(_) => {
            //         info!("Successfully sent TxnRecord to block exploror indexer");
            //     },
            //     Err(e) => {
            //         warn!("Error sending TxnRecord to block explorer indexer {}", e);
            //     },
            // }

            // info!("stufffffffffffff");

            match tls::run_server().await {
                Ok(_) => todo!(),
                Err(_) => todo!(),
            }
        });

        let handle = server.start(server_impl.into_rpc())?;

        Ok((handle, config.address))
    }
}

impl Default for JsonRpcServerConfig {
    fn default() -> JsonRpcServerConfig {
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9293);
        let mut vrrbdb_config = VrrbDbConfig::default();

        let temp_dir_path = std::env::temp_dir();
        let db_path = temp_dir_path.join(vrrb_core::helpers::generate_random_string());

        vrrbdb_config.path = db_path;

        let vrrbdb = VrrbDb::new(vrrbdb_config);
        let vrrbdb_read_handle = vrrbdb.read_handle();

        let mempool = LeftRightMempool::default();
        let mempool_read_handle_factory = mempool.factory();

        let node_type = NodeType::RPCNode;
        let (events_tx, _) = unbounded_channel();

        JsonRpcServerConfig {
            address,
            vrrbdb_read_handle,
            mempool_read_handle_factory,
            node_type,
            events_tx,
            // tls_config: None,
        }
    }
}
