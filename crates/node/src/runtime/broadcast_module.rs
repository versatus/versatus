use std::{collections::HashSet, net::SocketAddr, result::Result as StdResult};

use async_trait::async_trait;
use bytes::Bytes;
use network::{
    message::{Message, MessageBody},
    network::BroadcastEngine,
};
use primitives::{NodeType, PeerId};
use state::NodeStateReadHandle;
use telemetry::{error, info};
use tokio::{
    sync::{
        broadcast::{
            error::{RecvError, TryRecvError},
            Receiver,
        },
        mpsc::{channel, Receiver as MpscReceiver, Sender},
    },
    task::JoinHandle,
};
use uuid::Uuid;
use vrrb_core::event_router::{DirectedEvent, Event};

use crate::{NodeError, Result, RuntimeModule, RuntimeModuleState};

const BROADCAST_CONTROLLER_BUFFER_SIZE: usize = 10;

pub struct BroadcastModuleConfig {
    pub events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    pub node_type: NodeType,
    pub state_handle_factory: NodeStateReadHandle,
    pub udp_gossip_address_port: u16,
    pub raptorq_gossip_address_port: u16,
    pub node_id: PeerId,
}

// TODO: rename to GossipNetworkModule
pub struct BroadcastModule {
    // engine: BroadcastEngine,
    running_status: RuntimeModuleState,
    events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    state_handle_factory: NodeStateReadHandle,
    broadcast_handle: JoinHandle<Result<()>>,
    addr: SocketAddr,
    controller_rx: MpscReceiver<Event>,
}

impl BroadcastModule {
    pub async fn new(config: BroadcastModuleConfig) -> Result<Self> {
        let mut broadcast_engine = BroadcastEngine::new(
            config.udp_gossip_address_port,
            config.raptorq_gossip_address_port,
            32,
        )
        .await
        .map_err(|err| NodeError::Other(format!("unable to setup broadcast engine: {}", err)))?;

        let addr = broadcast_engine.local_addr();

        let (tx, controller_rx) =
            tokio::sync::mpsc::channel::<Event>(BROADCAST_CONTROLLER_BUFFER_SIZE);

        let mut bcast_controller = BroadcastEngineController::new(broadcast_engine);

        // NOTE: starts the listening loop
        let broadcast_handle = tokio::spawn(async move {
            let tx = tx.clone();

            bcast_controller.listen(tx).await
        });

        Ok(Self {
            events_tx: config.events_tx,
            running_status: RuntimeModuleState::Stopped,
            state_handle_factory: config.state_handle_factory,
            broadcast_handle,
            addr,
            controller_rx,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }
}

impl BroadcastModule {
    fn decode_event(&mut self, event: StdResult<Event, TryRecvError>) -> Event {
        match event {
            Ok(cmd) => cmd,
            Err(err) => match err {
                TryRecvError::Closed => {
                    error!("the events channel has been closed.");
                    Event::Stop
                },

                TryRecvError::Lagged(u64) => {
                    error!("receiver lagged behind");
                    Event::NoOp
                },
                _ => Event::NoOp,
            },
            _ => Event::NoOp,
        }
    }

    fn process_event(&mut self, event: Event) {
        match event {
            Event::BlockConfirmed(_) => {
                // do something
            },
            // Event::PeerJoined(_) => {
            //     // do something
            // },
            Event::PeerRequestedStateSync(peer_data) => {
                // do something
                self.state_handle_factory.values();
                // get a handle
                // copy state
                // turn it into bytes
                // send it to peer address
            },

            // Event::PeerRequestedStateSync(_) => {
            //     // do something
            // },
            Event::NoOp => {},
            _ => telemetry::warn!("unrecognized command received: {:?}", event),
        }
    }

    // fn handle_event_stream_input(&mut self, event: std::result::Result<Event,
    // RecvError>) {
    fn handle_event_stream_input(&mut self, event: Event) {
        info!("{} received {event:?}", self.name());

        dbg!(&event);

        // if let Ok(event) = event {
        if event == Event::Stop {
            info!("{0} received stop signal. Stopping", self.name());

            dbg!(&event);
            self.running_status = RuntimeModuleState::Terminating;
            if !self.broadcast_handle.is_finished() {
                self.broadcast_handle.abort();
            }

            return;
        }

        self.process_event(event);
        // }
    }
}

#[async_trait]
impl RuntimeModule for BroadcastModule {
    fn name(&self) -> String {
        String::from("Broadcast module")
    }

    fn status(&self) -> RuntimeModuleState {
        self.running_status.clone()
    }

    async fn start(&mut self, events_rx: &mut Receiver<Event>) -> Result<()> {
        info!("{0} started", self.name());

        // loop {
        //     tokio::select! {
        //         biased;
        //         Ok(event) = events_rx.recv() =>
        // self.handle_event_stream_input(event),         Some(controller_event)
        // = self.controller_rx.recv() => {             dbg!(controller_event);
        //         }
        //
        //     }
        // }

        while let Ok(event) = events_rx.recv().await {
            info!("{} received {event:?}", self.name());

            if event == Event::Stop {
                info!("{0} received stop signal. Stopping", self.name());

                self.running_status = RuntimeModuleState::Terminating;
                if !self.broadcast_handle.is_finished() {
                    self.broadcast_handle.abort();
                }

                break;
            }

            self.process_event(event);
        }

        self.running_status = RuntimeModuleState::Stopped;

        Ok(())
    }
}

#[derive(Debug)]
struct BroadcastEngineController {
    addr: SocketAddr,
    engine: BroadcastEngine,
}

impl BroadcastEngineController {
    pub fn new(engine: BroadcastEngine) -> Self {
        let addr = engine.local_addr();
        Self { engine, addr }
    }


    pub async fn listen(&mut self, tx: Sender<Event>) -> Result<()> {
        let listener = self.engine.get_incomming_connections();

        while let Some((conn, mut conn_incoming)) = listener.next().await {
            let res = conn_incoming.next().await.map_err(|err| {
                NodeError::Other(format!("unable to listen for new connections: {err}"))
            })?;

            let (_, _, raw_message) = res.unwrap_or((Bytes::new(), Bytes::new(), Bytes::new()));

            let message = Message::from(raw_message.to_vec());

            let body: MessageBody = message.data.into();

            if let Err(err) = tx.send(body.into()).await {
                error!("failed to forward data received from network: {err}");
            }
        }

        Ok(())
    }
}
