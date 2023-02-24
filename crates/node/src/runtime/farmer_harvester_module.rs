use std::{
    borrow::{Borrow, BorrowMut},
    collections::HashMap,
    hash::Hash,
    net::SocketAddr,
    path::PathBuf,
};

use async_trait::async_trait;
use block::TxnId;
use hbbft::crypto::{Signature, SignatureShare};
use indexmap::IndexMap;
use kademlia_dht::{Key, Node, NodeData};
use lr_trie::ReadHandleFactory;
use mempool::mempool::{LeftRightMempool, TxnStatus};
use patriecia::{db::MemoryDB, inner::InnerTrie};
use primitives::{GroupPublicKey, NodeIdx, PeerId, QuorumType, RawSignature, TxHashString};
use serde::{Deserialize, Serialize};
use signer::signer::{SignatureProvider, Signer};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, Message, TheaterError};
use tokio::sync::broadcast::error::TryRecvError;
use tracing::error;
use vrrb_core::{
    accountable::Accountable,
    bloom::Bloom,
    event_router::{DirectedEvent, Event, QuorumCertifiedTxn, Topic, Vote, VoteReceipt},
    txn::{TransactionDigest, Txn},
};

use crate::{result::Result, NodeError};

pub struct FarmerHarvesterModule {
    pub quorum_certified_txns: Option<Vec<QuorumCertifiedTxn>>,
    pub certified_txns_filter: Bloom,
    pub quorum_type: Option<QuorumType>,
    pub tx_mempool: Option<LeftRightMempool>,
    pub votes_pool: IndexMap<(TxHashString, String), Vec<Vote>>,
    pub group_public_key: GroupPublicKey,
    pub sig_provider: Option<SignatureProvider>,
    pub farmer_id: PeerId,
    pub farmer_node_idx: NodeIdx,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    clear_filter_rx: tokio::sync::mpsc::UnboundedReceiver<DirectedEvent>,
}

pub const PULL_TXN_BATCH_SIZE: usize = 100;

impl FarmerHarvesterModule {
    pub fn new(
        certified_txns_filter: Bloom,
        quorum_type: Option<QuorumType>,
        sig_provider: Option<SignatureProvider>,
        group_public_key: GroupPublicKey,
        farmer_id: PeerId,
        farmer_node_idx: NodeIdx,
        broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
        clear_filter_rx: tokio::sync::mpsc::UnboundedReceiver<DirectedEvent>,
    ) -> Self {
        let lrmpooldb = if let Some(QuorumType::Farmer) = quorum_type {
            Some(LeftRightMempool::new())
        } else {
            None
        };
        let quorum_certified_txns = if let Some(QuorumType::Farmer) = quorum_type {
            None
        } else {
            Some(Vec::new())
        };
        Self {
            quorum_certified_txns,
            certified_txns_filter,
            quorum_type,
            sig_provider,
            tx_mempool: lrmpooldb,
            status: ActorState::Stopped,
            label: String::from("FarmerHarvester"),
            id: uuid::Uuid::new_v4().to_string(),
            group_public_key,
            farmer_id,
            farmer_node_idx,
            broadcast_events_tx,
            clear_filter_rx,
            votes_pool: Default::default(),
        }
    }

    pub fn insert_txn(&mut self, txn: Txn) {
        if let Some(tx_mempool) = self.tx_mempool.borrow_mut() {
            let _ = tx_mempool.insert(txn);
        }
    }

    pub fn update_txn_status(&mut self, txn_id: TransactionDigest, status: TxnStatus) {
        if let Some(tx_mempool) = self.tx_mempool.borrow_mut() {
            let txn_record_opt = tx_mempool.get(&txn_id);
            if let Some(mut txn_record) = txn_record_opt {
                txn_record.status = status;
                self.remove_txn(txn_id);
                self.insert_txn(txn_record.txn);
            }
        }
    }

    pub fn remove_txn(&mut self, txn_id: TransactionDigest) {
        if let Some(tx_mempool) = self.tx_mempool.borrow_mut() {
            let _ = tx_mempool.remove(&txn_id);
        }
    }

    pub fn name(&self) -> String {
        String::from("FarmerHarvester module")
    }
}

#[async_trait]
impl Handler<Event> for FarmerHarvesterModule {
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
                    if let Some(tx_mempool) = self.tx_mempool.borrow_mut() {
                        let txns = tx_mempool.fetch_txns(1);
                        if let Some(sig_provider) = self.sig_provider.clone() {
                            for txn_record in txns.into_iter() {
                                let mut txn = txn_record.1.txn;
                                txn.receiver_farmer_id = Some(self.farmer_id.clone());
                                if let Ok(txn_bytes) = bincode::serialize(&txn) {
                                    if let Ok(signature) =
                                        sig_provider.generate_partial_signature(txn_bytes)
                                    {
                                        let vote = Vote {
                                            farmer_id: self.farmer_id.clone(),
                                            farmer_node_id: self.farmer_node_idx,
                                            signature,
                                            txn,
                                            quorum_public_key: self.group_public_key.clone(),
                                            quorum_threshold: 2,
                                        };
                                        self.broadcast_events_tx.send((Topic::Network, Event::Vote(vote.clone(), QuorumType::Farmer,2))).expect("Cannot send vote to broadcast channel to share votes among farmer nodes");
                                        self.broadcast_events_tx.send((Topic::Network, Event::Vote(vote, QuorumType::Harvester,2))).expect("Cannot send vote to broadcast channel to send vote to Harvester Node ");
                                    }
                                }
                            }
                        }
                    }
                }
            },
            Event::Vote(vote, quorum, quorum_threshold) => {
                if let QuorumType::Farmer = quorum {
                    if let Some(sig_provider) = self.sig_provider.clone() {
                        let txn = vote.txn;
                        let txn_bytes = bincode::serialize(&txn).unwrap();
                        let signature = sig_provider.generate_partial_signature(txn_bytes).unwrap();
                        let vote = Vote {
                            farmer_id: self.farmer_id.clone(),
                            farmer_node_id: self.farmer_node_idx,
                            signature,
                            txn,
                            quorum_public_key: self.group_public_key.clone(),
                            quorum_threshold,
                        };
                        self.broadcast_events_tx.send((Topic::Network,Event::Vote(vote,QuorumType::Harvester,2))).expect("Cannot send vote to broadcast channel to send vote to Harvester Node ");
                    }
                } else if let QuorumType::Harvester = quorum {
                    //Harvest should check for integrity of the vote by Voter( Does it vote truly
                    // comes from Voter Prevent Double Voting
                    if let Some(sig_provider) = self.sig_provider.clone() {
                        let farmer_quorum_key = hex::encode(vote.quorum_public_key.clone());
                        if let Some(mut votes) = self
                            .votes_pool
                            .get_mut(&(vote.txn.txn_id(), farmer_quorum_key.clone()))
                        {
                            let txn_id = vote.txn.txn_id();
                            if !self
                                .certified_txns_filter
                                .contains(&(txn_id.clone(), farmer_quorum_key.clone()))
                            {
                                votes.push(vote.clone());
                                if votes.len() >= quorum_threshold {
                                    let mut sig_shares = std::collections::BTreeMap::new();
                                    for v in votes.iter() {
                                        sig_shares.insert(v.farmer_node_id, v.signature.clone());
                                    }
                                    let result = sig_provider.generate_quorum_signature(sig_shares);
                                    if let Ok(threshold_signature) = result {
                                        if let Some(certified_pool) =
                                            self.quorum_certified_txns.borrow_mut()
                                        {
                                            let vote_receipts = votes
                                                .iter()
                                                .map(|v| VoteReceipt {
                                                    farmer_id: v.farmer_id.clone(),
                                                    farmer_node_id: v.farmer_node_id,
                                                    signature: v.signature.clone(),
                                                })
                                                .collect::<Vec<VoteReceipt>>();
                                            certified_pool.push(QuorumCertifiedTxn::new(
                                                vote.farmer_id.clone(),
                                                vote_receipts,
                                                vote.txn,
                                                threshold_signature,
                                            ));
                                        }
                                        let _ = self
                                            .certified_txns_filter
                                            .push(&(txn_id, farmer_quorum_key));
                                    }
                                }
                            }
                        } else {
                            self.votes_pool
                                .insert((vote.txn.txn_id(), farmer_quorum_key), vec![vote]);
                        }
                    }
                }
            },
            Event::PullQuorumCertifiedTxns(num_of_txns) => {
                if let Some(txns) = self.quorum_certified_txns.borrow() {
                    txns.iter().take(num_of_txns).for_each(|txn| {
                        self.broadcast_events_tx
                            .send((Topic::Storage, Event::QuorumCertifiedTxns(txn.clone())))
                            .expect("Failed to send Quorum Certified Txns");
                    });
                }
            },
            Event::NoOp => {},
            _ => {},
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
        process::exit,
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use dkg_engine::{test_utils, types::config::ThresholdConfig};
    use primitives::{NodeType, QuorumType::Farmer};
    use secp256k1::Message;
    use theater::ActorImpl;
    use vrrb_core::{
        cache,
        event_router::{DirectedEvent, Event, PeerData},
        is_enum_variant,
        keypair::KeyPair,
        txn::NewTxnArgs,
    };

    use super::*;

    #[tokio::test]
    async fn farmer_harvester_runtime_module_starts_and_stops() {
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (broadcast_events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (_, clear_filter_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let mut farmer_harvester_swarm_module = FarmerHarvesterModule::new(
            Bloom::new(10000),
            None,
            None,
            vec![],
            vec![],
            0,
            broadcast_events_tx,
            clear_filter_rx,
        );
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
        let (_, clear_filter_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();

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
        let mut farmer_harvester_swarm_module = FarmerHarvesterModule::new(
            Bloom::new(10000),
            Some(Farmer),
            Some(sig_provider),
            group_public_key,
            dkg_engine.secret_key.public_key().to_bytes().to_vec(),
            1,
            broadcast_events_tx,
            clear_filter_rx,
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
                timestamp: 0,
                sender_address: String::from("aaa1"),
                sender_public_key: keypair.get_miner_public_key().clone(),
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
        if let Some(tx_mempool) = farmer_harvester_swarm_module.tx_mempool.borrow_mut() {
            let _ = tx_mempool.extend(txns);
        }
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

    #[tokio::test]
    async fn farmer_harvester_harvest_votes() {
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();

        let mut dkg_engines = test_utils::generate_dkg_engine_with_states().await;
        let mut farmers = vec![];
        let mut broadcast_rxs = vec![];
        while dkg_engines.len() > 0 {
            let (_, clear_filter_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
            let (broadcast_events_tx, mut broadcast_events_rx) =
                tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
            broadcast_rxs.push(broadcast_events_rx);
            let dkg_engine = dkg_engines.pop().unwrap();
            let group_public_key = dkg_engine
                .dkg_state
                .public_key_set
                .clone()
                .unwrap()
                .public_key()
                .to_bytes()
                .to_vec();
            let mut farmer = FarmerHarvesterModule::new(
                Bloom::new(10000),
                Some(Farmer),
                Some(SignatureProvider {
                    dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine.dkg_state)),
                    quorum_config: ThresholdConfig {
                        threshold: 2,
                        upper_bound: 4,
                    },
                }),
                group_public_key,
                dkg_engine.secret_key.public_key().to_bytes().to_vec(),
                dkg_engine.node_idx,
                broadcast_events_tx,
                clear_filter_rx,
            );
            farmer.quorum_certified_txns = Some(Vec::<QuorumCertifiedTxn>::new());
            farmers.push(farmer);
        }

        let keypair = KeyPair::random();
        let mut txns = HashSet::<Txn>::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let sender_address = String::from("aaa1");
        let receiver_address = String::from("bbb1");
        let txn_amount: u128 = 1010101;

        for n in 0..1 {
            let mut txn = Txn::new(NewTxnArgs {
                timestamp: 0,
                sender_address: String::from("aaa1"),
                sender_public_key: keypair.get_miner_public_key().clone(),
                receiver_address: receiver_address.clone(),
                token: None,
                amount: txn_amount + n,
                payload: Some(String::from("x")),
                signature: vec![],
                validators: Some(HashMap::<String, bool>::new()),
                nonce: 0,
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

        let mut ctrx_txns = vec![];
        let mut handles = vec![];
        for mut farmer in farmers.into_iter() {
            let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

            if let Some(tx_mempool) = farmer.tx_mempool.borrow_mut() {
                let _ = tx_mempool.extend(txns.clone());
            }
            let mut farmer_harvester_swarm_module = ActorImpl::new(farmer);

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
            ctrx_txns.push(ctrl_tx);
            handles.push(handle);
        }

        for ctrl_tx in ctrx_txns.iter() {
            ctrl_tx.send(Event::Farm.into()).unwrap();
        }
        for broadcast_events_rx in broadcast_rxs.iter_mut() {
            let event = broadcast_events_rx.recv().await.unwrap();
            assert_eq!(event.0, Topic::Network);
            if let Event::Vote(vote, quorum_type, quorum_threshold) = event.1 {
                if quorum_type == Farmer {
                    let c = ctrx_txns.get(0).unwrap().send(Event::Vote(
                        vote,
                        QuorumType::Harvester,
                        quorum_threshold,
                    ));
                }
            }
        }
        ctrx_txns
            .get(0)
            .unwrap()
            .send(Event::PullQuorumCertifiedTxns(1))
            .unwrap();

        for ctrl_tx in ctrx_txns.iter() {
            ctrl_tx.send(Event::Stop.into()).unwrap();
        }

        let _ = broadcast_rxs.get_mut(0).unwrap().recv().await;
        let event = broadcast_rxs.get_mut(0).unwrap().recv().await;
        if let Some(event) = event {
            if let Topic::Storage = event.0 {
                is_enum_variant!(event.1, Event::QuorumCertifiedTxns { .. });
            }
        }

        for h in handles.into_iter() {
            h.await.unwrap();
        }
    }
}
