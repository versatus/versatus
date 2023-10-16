use async_trait::async_trait;
use events::{Event, EventMessage};
use telemetry::info;
use theater::{ActorId, ActorLabel, ActorState, Handler, TheaterError};
use primitives::RUNTIME_TOPIC_STR;

use super::NetworkModule;

#[async_trait]
impl Handler<EventMessage> for NetworkModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        format!("Network::{}", self.id())
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::PeerJoined(peer_data) => {
                info!("Storing peer information from {} in DHT", peer_data.node_id);

                // TODO: revisit this insert method
                self.kademlia_node.insert(
                    peer_data.kademlia_peer_id,
                    &peer_data.kademlia_liveness_addr.to_string(),
                );

                let evt = Event::NodeAddedToPeerList(peer_data.clone());
                let em = EventMessage::new(Some(RUNTIME_TOPIC_STR.into()), evt);

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::QuorumMembershipAssigmentCreated(assigned_membership) => {
                self.notify_quorum_membership_assignment(assigned_membership)
                    .await?;
            },

            Event::ClaimCreated(claim) => {
                info!("Broadcasting claim to peers");
                self.broadcast_claim(claim).await?;
            },

            Event::PartCommitmentCreated(node_id, part) => {
                info!("Broadcasting part commitment to peers in quorum");
                self.broadcast_part_commitment(node_id, part).await?;
            },

            Event::PartCommitmentAcknowledged {
                node_id,
                sender_id,
                ack,
            } => {
                info!("Broadcasting part commitment acknowledgement to peers in quorum");
                self.broadcast_part_commitment_acknowledgement(node_id, sender_id, ack)
                    .await?;
            },

            Event::ConvergenceBlockCertified(block) => {
                info!("Broadcasting certified convergence block to network");
                self.broadcast_certified_convergence_block(block).await?;
            },
            Event::ConvergenceBlockPartialSignComplete(sig) => {
                info!("Broadcasting partial signature of convergence block to network");
                self.broadcast_convergence_block_partial_signature(sig)
                    .await?;
            },
            Event::Stop => {
                // NOTE: stop the kademlia node instance
                self.node_ref().kill();
                return Ok(ActorState::Stopped);
            },
            Event::BroadcastCertificate(cert) => {
                info!("Broadcasting certificate to network");
                self.broadcast_certificate(cert).await?;
            },
            Event::BroadcastTransactionVote(vote) => {
                info!("Broadcasting transaction vote to network");
                self.broadcast_transaction_vote(vote).await?;
            },
            _ => {},
        }

        Ok(ActorState::Running)
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.label(),
            self.id(),
        );
    }
}
