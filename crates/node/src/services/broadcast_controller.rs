use std::{collections::HashSet, net::SocketAddr, result::Result as StdResult};

use async_trait::async_trait;
use bytes::Bytes;
use network::{
    message::{Message, MessageBody},
    network::BroadcastEngine,
};
use primitives::{NodeType, PeerId};
use telemetry::{error, info, warn};
use theater::{ActorLabel, ActorState, Handler};
use tokio::{
    sync::{
        broadcast::{
            error::{RecvError, TryRecvError},
            Receiver,
        },
        mpsc::Sender,
    },
    task::JoinHandle,
};
use uuid::Uuid;
use vrrb_core::event_router::{DirectedEvent, Event};

use crate::{NodeError, Result, RuntimeModule, RuntimeModuleState};

pub const BROADCAST_CONTROLLER_BUFFER_SIZE: usize = 10000;

#[derive(Debug)]
pub struct BroadcastEngineController {
    addr: SocketAddr,
    engine: BroadcastEngine,
}

impl BroadcastEngineController {
    pub fn new(engine: BroadcastEngine) -> Self {
        let addr = engine.local_addr();
        Self { engine, addr }
    }

    pub async fn listen(&mut self, tx: Sender<Event>, rx: Receiver<Event>) -> Result<()> {
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
