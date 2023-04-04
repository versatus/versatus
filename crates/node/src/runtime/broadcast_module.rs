use std::{net::SocketAddr, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use events::{DirectedEvent, Event};
use network::network::BroadcastEngine;
use primitives::{NodeType, PeerId};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::{error, instrument};
use theater::{ActorLabel, ActorState, Handler};
use tokio::sync::mpsc::unbounded_channel;
use uuid::Uuid;

use crate::{
    broadcast_controller::{self, BroadcastEngineController, BroadcastEngineControllerConfig},
    NodeError,
    Result,
};

pub struct BroadcastModuleConfig {
    pub events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    pub node_type: NodeType,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub udp_gossip_address_port: u16,
    pub raptorq_gossip_address_port: u16,
    pub node_id: PeerId,
}

// TODO: rename to GossipNetworkModule
#[derive(Debug)]
pub struct BroadcastModule {
    id: Uuid,
    status: ActorState,
    events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    vrrbdb_read_handle: VrrbDbReadHandle,
    engine_controller_handle: tokio::task::JoinHandle<()>,
    engine_controller_tx: tokio::sync::mpsc::UnboundedSender<Event>,
    broadcast_engine_local_addr: SocketAddr,
}

/// Useful alias to represent get_incomming_connections' return type
type BytesTrifecta = (Bytes, Bytes, Bytes);

const PACKET_TIMEOUT_DURATION: u64 = 10;
const EMPTY_BYTES_TRIFECTA: BytesTrifecta = (Bytes::new(), Bytes::new(), Bytes::new());

trait Timeout: Sized {
    fn timeout(self) -> tokio::time::Timeout<Self>;
}

impl<F: std::future::Future> Timeout for F {
    fn timeout(self) -> tokio::time::Timeout<Self> {
        tokio::time::timeout(Duration::from_secs(PACKET_TIMEOUT_DURATION), self)
    }
}

impl BroadcastModule {
    pub async fn new(config: BroadcastModuleConfig) -> Result<Self> {
        let broadcast_engine = BroadcastEngine::new(config.udp_gossip_address_port, 32)
            .await
            .map_err(|err| {
                NodeError::Other(format!("unable to setup broadcast engine: {:?}", err))
            })?;

        let broadcast_engine_local_addr = broadcast_engine.local_addr();

        let events_tx = config.events_tx.clone();

        let (engine_controller_tx, engine_controller_rx) = unbounded_channel();

        let engine_controller_handle = tokio::spawn(async move {
            let events_tx = events_tx;

            let mut broadcast_engine = broadcast_engine;

            let mut broadcast_controller =
                BroadcastEngineController::new(BroadcastEngineControllerConfig {
                    engine: broadcast_engine,
                    events_tx,
                })
                .listen(engine_controller_rx)
                .await;
        });

        Ok(Self {
            id: Uuid::new_v4(),
            events_tx: config.events_tx,
            status: ActorState::Stopped,
            vrrbdb_read_handle: config.vrrbdb_read_handle,
            broadcast_engine_local_addr,
            engine_controller_tx,
            engine_controller_handle,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.broadcast_engine_local_addr
    }

    pub fn name(&self) -> String {
        "BroadcastModule".to_string()
    }
}

/// The number of erasures that the raptorq encoder will use to encode the
/// block.
const RAPTOR_ERASURE_COUNT: u32 = 3000;

#[async_trait]
impl Handler<Event> for BroadcastModule {
    fn id(&self) -> theater::ActorId {
        self.id.to_string()
    }

    fn label(&self) -> ActorLabel {
        self.name()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    #[instrument]
    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        if let Err(err) = self.engine_controller_tx.send(event.clone()) {
            error!("unable to send event to broadcast controller: {:?}", err);

            return Ok(ActorState::Stopped);
        }

        if matches!(event, Event::Stop) {
            return Ok(ActorState::Stopped);
        }

        Ok(ActorState::Running)
    }
}

#[cfg(test)]
mod tests {
    use std::io::stdout;

    use events::{Event, SyncPeerData};
    use primitives::NodeType;
    use serial_test::serial;
    use storage::vrrbdb::{VrrbDb, VrrbDbConfig};
    use telemetry::TelemetrySubscriber;
    use theater::{Actor, ActorImpl};
    use tokio::{net::UdpSocket, sync::mpsc::unbounded_channel};

    use super::{BroadcastModule, BroadcastModuleConfig};

    #[tokio::test]
    #[serial]
    async fn test_broadcast_module() {
        let (internal_events_tx, mut internal_events_rx) = unbounded_channel();

        let node_id = uuid::Uuid::new_v4().to_string().into_bytes();

        ////////////////////////////////////////////////////////////////

        // let temp_dir_path = std::env::temp_dir();
        // let state_backup_path =
        // temp_dir_path.join(vrrb_core::helpers::generate_random_string());

        // let db = VrrbDb::new(VrrbDbConfig {
        //     path: state_backup_path,
        //     state_store_path: None,
        //     transaction_store_path: None,
        //     event_store_path: None,
        // });

        ////////////////////////////////////////////////////////////////

        let mut db_config = VrrbDbConfig::default();

        let temp_dir_path = std::env::temp_dir();
        let db_path = temp_dir_path.join(vrrb_core::helpers::generate_random_string());

        db_config.with_path(db_path);

        let db = VrrbDb::new(db_config);

        ////////////////////////////////////////////////////////////////

        let vrrbdb_read_handle = db.read_handle();

        let config = BroadcastModuleConfig {
            events_tx: internal_events_tx,
            vrrbdb_read_handle,
            node_type: NodeType::Full,
            udp_gossip_address_port: 0,
            raptorq_gossip_address_port: 0,
            node_id,
        };

        let (events_tx, mut events_rx) = tokio::sync::broadcast::channel::<Event>(10);

        let broadcast_module = BroadcastModule::new(config).await.unwrap();

        let mut broadcast_module_actor = ActorImpl::new(broadcast_module);

        let handle = tokio::spawn(async move {
            broadcast_module_actor.start(&mut events_rx).await.unwrap();
        });

        let bound_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

        let address = bound_socket.local_addr().unwrap();

        let peer_data = SyncPeerData {
            address,
            raptor_udp_port: 9993,
            quic_port: 9994,
            node_type: NodeType::Full,
        };

        events_tx.send(Event::SyncPeers(vec![peer_data])).unwrap();
        events_tx.send(Event::Stop).unwrap();

        let evt = internal_events_rx.recv().await.unwrap();

        handle.await.unwrap();
    }
}
