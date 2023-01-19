use std::{hash::Hash, net::SocketAddr, path::PathBuf};

use async_trait::async_trait;
use hbbft::crypto::SignatureShare;
use kademlia_dht::{Key, Node, NodeData};
use lr_trie::ReadHandleFactory;
use mempool::mempool::{LeftRightMempool, TxnStatus};
use patriecia::{db::MemoryDB, inner::InnerTrie};
use primitives::{GroupPublicKey, PeerId, QuorumType, RawSignature, TxHashString};
use serde::{Deserialize, Serialize};
use signer::signer::{SignatureProvider, Signer};
use state::{NodeState, NodeStateConfig, NodeStateReadHandle};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, Message, TheaterError};
use tokio::sync::broadcast::error::TryRecvError;
use tracing::error;
use vrrb_core::{
    event_router::{DirectedEvent, Event, Topic, Vote},
    txn::Txn,
};

use crate::{result::Result, NodeError};

#[derive(Clone)]
pub struct FarmerHarvestModule {
    pub quorum_type: Option<QuorumType>,
    pub tx_mempool: LeftRightMempool,
    pub group_public_key: GroupPublicKey,
    pub sig_provider: Option<SignatureProvider>,
    pub farmer_id: PeerId,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

pub const PULL_TXN_BATCH_SIZE: usize = 10;


#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct QuorumCert {
    farmer_id: Vec<u8>,
    /// All valid votes
    votes: Vec<Vote>,
    /// Threshold Signature
    signature: RawSignature,
}
impl FarmerHarvestModule {
    pub fn new(
        quorum_type: Option<QuorumType>,
        sig_provider: Option<SignatureProvider>,
        group_public_key: GroupPublicKey,
        farmer_id: PeerId,
        broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    ) -> Self {
        let lrmpooldb = LeftRightMempool::new();
        Self {
            quorum_type,
            sig_provider,
            tx_mempool: lrmpooldb,
            status: ActorState::Stopped,
            label: String::from("FarmerHarvester"),
            id: uuid::Uuid::new_v4().to_string(),
            group_public_key,
            farmer_id,
            broadcast_events_tx,
        }
    }

    pub fn insert_txn(&mut self, txn: Txn) {
        self.tx_mempool.insert(txn);
    }

    pub fn update_txn_status(&mut self, txn_id: TxHashString, status: TxnStatus) {
        let txn_record_opt = self.tx_mempool.get(&txn_id);
        if let Some(mut txn_record) = txn_record_opt {
            txn_record.status = status;
            self.remove_txn(txn_id);
            self.insert_txn(txn_record.txn);
        }
    }

    pub fn remove_txn(&mut self, txn_id: TxHashString) {
        self.tx_mempool.remove(&txn_id);
    }

    pub fn vote_txn(&mut self, txn_id: TxHashString) {
        self.tx_mempool.remove(&txn_id);
    }

    pub fn name(&self) -> String {
        String::from("FarmerHarvester module")
    }
}

#[async_trait]
impl Handler<Event> for FarmerHarvestModule {
    fn id(&self) -> ActorId {
        self.id.clone()
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

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.name(),
            self.label()
        );
    }

    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            Event::Farm => {
                if let Some(QuorumType::Farmer) = self.quorum_type {
                    let txns = self.tx_mempool.fetch_txns(1);
                    if let Some(sig_provider) = self.sig_provider.clone() {
                        for txn_record in txns.into_iter() {
                            let txn = txn_record.1.txn;
                            let txn_bytes = bincode::serialize(&txn).unwrap();
                            let signature =
                                sig_provider.generate_partial_signature(txn_bytes).unwrap();
                            let vote = Vote {
                                farmer_id: self.farmer_id.clone(),
                                signature,
                                txn,
                                quorum_public_key: self.group_public_key.clone(),
                            };
                            self.broadcast_events_tx.send((Topic::Network, Event::Vote(vote.clone(), QuorumType::Farmer))).expect("Cannot send vote to broadcast channel to share votes among farmer nodes");
                            self.broadcast_events_tx.send((Topic::Network,Event::Vote(vote,QuorumType::Harvester))).expect("Cannot send vote to broadcast channel to send vote to Harvester Node ");
                        }
                    }
                }
            },
            Event::NoOp => {},
            _ => telemetry::warn!("Unrecognized command received: {:?}", event),
        }

        Ok(ActorState::Running)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        env,
        net::{IpAddr, Ipv4Addr},
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use dkg_engine::{test_utils, types::config::ThresholdConfig};
    use primitives::{is_enum_variant, NodeType, QuorumType::Farmer};
    use secp256k1::Message;
    use theater::ActorImpl;
    use vrrb_core::{
        event_router::{DirectedEvent, Event, PeerData},
        keypair::KeyPair,
        txn::NewTxnArgs,
    };

    use super::*;

    #[tokio::test]
    async fn farmer_harvester_runtime_module_starts_and_stops() {
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (broadcast_events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let mut farmer_harvester_swarm_module =
            FarmerHarvestModule::new(None, None, vec![], vec![], broadcast_events_tx);
        let mut farmer_harvester_swarm_module = ActorImpl::new(farmer_harvester_swarm_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(farmer_harvester_swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            farmer_harvester_swarm_module
                .start(&mut ctrl_rx)
                .await
                .unwrap();
            assert_eq!(
                farmer_harvester_swarm_module.status(),
                ActorState::Terminating
            );
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();
    }


    #[tokio::test]
    async fn farmer_harvester_farm_cast_vote() {
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (broadcast_events_tx, mut broadcast_events_rx) =
            tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let mut dkg_engines = test_utils::generate_dkg_engine_with_states().await;
        let dkg_engine = dkg_engines.pop().unwrap();
        let group_public_key = dkg_engine
            .dkg_state
            .public_key_set
            .clone()
            .unwrap()
            .public_key()
            .to_bytes()
            .to_vec();
        let sig_provider = SignatureProvider {
            dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine.dkg_state)),
            quorum_config: ThresholdConfig {
                threshold: 2,
                upper_bound: 4,
            },
        };
        let mut farmer_harvester_swarm_module = FarmerHarvestModule::new(
            Some(Farmer),
            Some(sig_provider),
            group_public_key,
            dkg_engine.secret_key.public_key().to_bytes().to_vec(),
            broadcast_events_tx,
        );
        let keypair = KeyPair::random();
        let mut txns = HashSet::<Txn>::new();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        // let txn_id = String::from("1");
        let sender_address = String::from("aaa1");
        let receiver_address = String::from("bbb1");
        let txn_amount: u128 = 1010101;

        for n in 1..101 {
            let mut txn = Txn::new(NewTxnArgs {
                sender_address: String::from("aaa1"),
                sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
                receiver_address: receiver_address.clone(),
                token: None,
                amount: txn_amount + n,
                payload: Some(String::from("x")),
                validators: Some(HashMap::<String, bool>::new()),
                nonce: 0,
                signature: vec![],
            });
            let bytes = bincode::serialize(&txn).unwrap();
            let sig = hex::decode(
                keypair
                    .miner_kp
                    .0
                    .sign_ecdsa(
                        Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(&bytes),
                    )
                    .to_string(),
            )
            .unwrap();
            txn.signature = Some(sig);
            txns.insert(txn);
        }
        farmer_harvester_swarm_module.tx_mempool.extend(txns);
        let mut farmer_harvester_swarm_module = ActorImpl::new(farmer_harvester_swarm_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(farmer_harvester_swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            farmer_harvester_swarm_module
                .start(&mut ctrl_rx)
                .await
                .unwrap();
            assert_eq!(
                farmer_harvester_swarm_module.status(),
                ActorState::Terminating
            );
        });

        ctrl_tx.send(Event::Farm.into()).unwrap();
        let event = broadcast_events_rx.recv().await.unwrap();
        assert_eq!(event.0, Topic::Network);
        is_enum_variant!(event.1, Event::Vote { .. });
        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();
    }
}
