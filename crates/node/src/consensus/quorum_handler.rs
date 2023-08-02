use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use block::header::BlockHeader;
use ethereum_types::U256;
use events::{Event, EventMessage, EventPublisher, EventSubscriber, PeerData};
use primitives::NodeId;
use quorum::{
    election::Election,
    quorum::{Quorum, QuorumError},
};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler, TheaterError};
use vrrb_config::{NodeConfig, QuorumMember};
use vrrb_core::claim::{Claim, Eligibility};

use crate::{consensus::QuorumModule, NodeError, RuntimeComponent, RuntimeComponentHandle};

#[async_trait]
impl Handler<EventMessage> for QuorumModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        format!("QuorumModule::{}", self.id())
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_start(&self) {
        info!("{} starting", self.label());
    }

    fn on_stop(&self) {
        info!("{} received stop signal. Stopping", self.label());
    }

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::NodeAddedToPeerList(peer_data) => {
                if let Some(quorum_config) = self.bootstrap_quorum_config.clone() {
                    let node_id = peer_data.node_id.clone();

                    let quorum_member_ids = quorum_config
                        .membership_config
                        .quorum_members
                        .iter()
                        .cloned()
                        .map(|membership| membership.member.node_id)
                        .collect::<Vec<NodeId>>();

                    if quorum_member_ids.contains(&node_id) {
                        self.bootstrap_quorum_available_nodes
                            .insert(node_id, (peer_data, true));
                    }

                    let available_nodes = self.bootstrap_quorum_available_nodes.clone();
                    let all_nodes_available =
                        available_nodes.iter().all(|(_, (_, is_online))| *is_online);

                    if all_nodes_available {
                        info!("All quorum members are online. Triggering genesis quorum elections");

                        if matches!(self.node_config.node_type, primitives::NodeType::Bootstrap) {
                            self.assign_peer_list_to_quorums(available_nodes)
                                .await
                                .map_err(|err| TheaterError::Other(err.to_string()))?;
                        }
                    }
                }
            },

            // TODO: refactor these event handlers to properly match architecture
            // Event::QuorumElection(header) => {
            //     let claims = self.vrrbdb_read_handle.claim_store_values();
            //
            //     if let Ok(quorum) = self.elect_quorum(claims, header) {
            //         if let Err(err) = self
            //             .events_tx
            //             .send(Event::ElectedQuorum(quorum).into())
            //             .await
            //         {
            //             telemetry::error!("{}", err);
            //         }
            //     }
            // },
            // Event::MinerElection(header) => {
            //     let claims = self.vrrbdb_read_handle.claim_store_values();
            //     let mut election_results: BTreeMap<U256, Claim> =
            //         self.elect_miner(claims, header.block_seed);
            //
            //     let winner = Self::get_winner(&mut election_results);
            //
            //     if let Err(err) = self
            //         .events_tx
            //         .send(Event::ElectedMiner(winner).into())
            //         .await
            //     {
            //         telemetry::error!("{}", err);
            //     }
            // },
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            _ => {},
        }

        Ok(ActorState::Running)
    }
}
