use std::{collections::HashSet, net::SocketAddr, result::Result as StdResult};

use async_trait::async_trait;
use bytes::Bytes;
use events::{DirectedEvent, Event};
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

use crate::{NodeError, Result, RuntimeModule, RuntimeModuleState};

pub const BROADCAST_CONTROLLER_BUFFER_SIZE: usize = 10000;

/// The number of erasures that the raptorq encoder will use to encode the
/// block.
const RAPTOR_ERASURE_COUNT: u32 = 3000;

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

    pub async fn listen(&self) {
        loop {
            // TODO: refactor this loop and these functions to adapt to tokio select
            tokio::select! {
                Ok(v) = self.listen_for_network_events().await => self.handle_network_event(v).await,
                Some(v) = self.listen_for_internal_events().await => self.handle_internal_event(v).await,
                else => break,
            }
        }
    }

    async fn handle_network_event(&self, message: Message) -> Option<()> {
        todo!()
    }

    async fn handle_internal_event(&self, event: Event) -> Option<()> {
        todo!()
    }

    /// Turns connection data into message then returns it
    async fn listen_for_network_events(&mut self) -> Result<Message> {
        if let Some((conn, mut conn_incoming)) =
            self.engine.get_incomming_connections().next().await
        {
            let res = conn_incoming.next().await.map_err(|err| {
                NodeError::Other(format!("unable to listen for new connections: {err}"))
            })?;

            let (_, _, raw_message) = res.unwrap_or((Bytes::new(), Bytes::new(), Bytes::new()));
            let message = Message::from(raw_message.to_vec());
            return Ok(message);
        }

        Err(NodeError::Other("No message received".to_string()))
    }

    async fn listen_for_internal_events(&mut self) -> Result<Event> {
        todo!();
        // Err(NodeError::Other("No event received".to_string()))
    }

    async fn listen_for_network_events_(
        &mut self,
        tx: Sender<Event>,
        rx: Receiver<Event>,
    ) -> Result<()> {
        let listener = self.engine.get_incomming_connections();

        while let Some((conn, mut conn_incoming)) = listener.next().await {
            let res = conn_incoming.next().await.map_err(|err| {
                NodeError::Other(format!("unable to listen for new connections: {err}"))
            })?;

            let (_, _, raw_message) = res.unwrap_or((Bytes::new(), Bytes::new(), Bytes::new()));

            let message = Message::from(raw_message.to_vec());

            // let body = message.data;

            match message.data {
                MessageBody::InvalidBlock { .. } => {},
                MessageBody::Disconnect { .. } => {},
                MessageBody::StateComponents { .. } => {},
                MessageBody::Genesis { .. } => {},
                MessageBody::Child { .. } => {},
                MessageBody::Parent { .. } => {},
                MessageBody::Ledger { .. } => {},
                MessageBody::NetworkState { .. } => {},
                MessageBody::ClaimAbandoned { .. } => {},
                MessageBody::ResetPeerConnection { .. } => {},
                MessageBody::RemovePeer { .. } => {},
                MessageBody::AddPeer { .. } => {},
                MessageBody::DKGPartCommitment {
                    part_commitment,
                    sender_id,
                } => {},
                MessageBody::DKGPartAcknowledgement { .. } => {},
                MessageBody::Vote { .. } => {},
                MessageBody::Empty => {},
            }
            // if let Err(err) = tx.send(body.into()).await {
            //     error!("failed to forward data received from network:
            // {err}"); }
        }

        Ok(())
    }

    async fn listen_to_internal_events(&self, event: Event) -> Result<()> {
        match event {
            Event::Stop => Ok(()),
            Event::PartMessage(sender_id, part_commitment) => {
                let status = self
                    .engine
                    .quic_broadcast(Message::new(MessageBody::DKGPartCommitment {
                        sender_id,
                        part_commitment,
                    }))
                    .await?;
                // .map_err(|err| {
                //     error!("Error occured while broadcasting ack commitment to peers:
                // {err}");     TheaterError::Other(err.to_string())
                // })?;

                info!("Broadcasted part commitment to peers: {status:?}");
                Ok(())
            },
            Event::SyncPeers(peers) => {
                if peers.is_empty() {
                    warn!("No peers to sync with");
                    // TODO: revisit this return
                    return Ok(());
                }

                let mut quic_addresses = vec![];
                let mut raptor_peer_list = vec![];

                for peer in peers.iter() {
                    let addr = peer.address;

                    quic_addresses.push(addr);

                    let mut raptor_addr = addr;
                    raptor_addr.set_port(peer.raptor_udp_port);
                    raptor_peer_list.push(raptor_addr);
                }

                self.engine.add_raptor_peers(raptor_peer_list);

                let status = self.engine.add_peer_connection(quic_addresses).await?;

                info!("{status:?}");
                Ok(())
            },
            Event::Vote(vote, quorum_type, farmer_quorum_threshold) => {
                let status = self
                    .engine
                    .quic_broadcast(Message::new(MessageBody::Vote {
                        vote,
                        quorum_type,
                        farmer_quorum_threshold,
                    }))
                    .await?;
                info!("{status:?}");
                Ok(())
            },
            // Broadcasting the Convergence block to the peers.
            Event::BlockConfirmed(block) => {
                let status = self
                    .engine
                    .unreliable_broadcast(block, RAPTOR_ERASURE_COUNT, self.engine.raptor_udp_port)
                    .await?;

                info!("{status:?}");
                Ok(())
            },
            _ => Ok(()),
        }
    }
}

//
// pub async fn process_received_msg(engine: &mut BroadcastEngine) -> Result<()>
// {     loop {
//         let (_, mut incoming) = engine
//             .get_incomming_connections()
//             .next()
//             .await
//             .ok_or_else(|| NodeError::Other("unable to get incoming
// connections".to_string()))?;
//
//         let (_, _, msg_bytes) = incoming
//             .next()
//             .timeout()
//             .await
//             .unwrap_or(Ok(None))
//             .unwrap_or_default()
//             .unwrap_or(EMPTY_BYTES_TRIFECTA);
//
//         let msg = Message::from_bytes(&msg_bytes);
//
//         match msg.data {
//             MessageBody::InvalidBlock { .. } => {},
//             MessageBody::Disconnect { .. } => {},
//             MessageBody::StateComponents { .. } => {},
//             MessageBody::Genesis { .. } => {},
//             MessageBody::Child { .. } => {},
//             MessageBody::Parent { .. } => {},
//             MessageBody::Ledger { .. } => {},
//             MessageBody::NetworkState { .. } => {},
//             MessageBody::ClaimAbandoned { .. } => {},
//             MessageBody::ResetPeerConnection { .. } => {},
//             MessageBody::RemovePeer { .. } => {},
//             MessageBody::AddPeer { .. } => {},
//             MessageBody::DKGPartCommitment {
//                 part_commitment,
//                 sender_id,
//             } => {},
//             MessageBody::DKGPartAcknowledgement { .. } => {},
//             MessageBody::Vote { .. } => {},
//             MessageBody::Empty => {},
//         }
//     }
// }
