use std::net::SocketAddr;

use async_trait::async_trait;
use jsonrpsee::{
    core::Error,
    server::{ServerBuilder, SubscriptionSink},
    types::SubscriptionResult,
};
use mempool::MempoolReadHandleFactory;
use primitives::NodeType;
use storage::vrrbdb::VrrbDbReadHandle;
use tokio::sync::mpsc::UnboundedSender;
use vrrb_core::{
    account::Account,
    event_router::{DirectedEvent, Event, Topic},
    txn::NewTxnArgs,
};

use super::api::{CreateTxnArgs, FullMempoolSnapshot};
use crate::rpc::{
    api::{FullStateSnapshot, RpcServer},
    server_impl::RpcServerImpl,
};

#[derive(Debug, Clone)]
pub struct JsonRpcServerConfig {
    pub address: SocketAddr,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub node_type: NodeType,
    pub events_tx: UnboundedSender<DirectedEvent>,
}

#[derive(Debug)]
pub struct JsonRpcServer;

impl JsonRpcServer {
    pub async fn run(config: &JsonRpcServerConfig) -> anyhow::Result<SocketAddr> {
        let server = ServerBuilder::default().build(config.address).await?;

        let server_impl = RpcServerImpl {
            node_type: config.node_type,
            events_tx: config.events_tx.clone(),
            vrrbdb_read_handle: config.vrrbdb_read_handle.clone(),
            mempool_read_handle_factory: config.mempool_read_handle_factory.clone(),
        };

        let addr = server.local_addr()?;
        let handle = server.start(server_impl.into_rpc())?;

        // TODO: refactor example out of here
        // In this example we don't care about doing shutdown so let's it run forever.
        // You may use the `ServerHandle` to shut it down or manage it yourself.
        tokio::spawn(handle.stopped());

        Ok(addr)
    }
}
