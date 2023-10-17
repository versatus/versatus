use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use events::{EventPublisher, DEFAULT_BUFFER};
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use primitives::NodeType;
use storage::vrrbdb::{VrrbDb, VrrbDbConfig, VrrbDbReadHandle};
use tokio::sync::mpsc::channel;

use crate::rpc::{api::RpcApiServer, server_impl::RpcServerImpl};

#[derive(Debug, Clone)]
pub struct JsonRpcServerConfig {
    pub address: SocketAddr,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub node_type: NodeType,
    pub events_tx: EventPublisher,
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

        let addr = server.local_addr()?;
        let handle = server.start(server_impl.into_rpc())?;

        // TODO: refactor example out of here
        // In this example we don't care about doing shutdown so let's it run forever.
        // You may use the `ServerHandle` to shut it down or manage it yourself.
        Ok((handle, addr))
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

        let node_type = NodeType::Full;
        let (events_tx, _) = channel(DEFAULT_BUFFER);

        JsonRpcServerConfig {
            address,
            vrrbdb_read_handle,
            mempool_read_handle_factory,
            node_type,
            events_tx,
        }
    }
}
