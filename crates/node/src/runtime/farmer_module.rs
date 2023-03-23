use std::{
    borrow::{Borrow, BorrowMut},
    thread,
};

use async_trait::async_trait;
use crossbeam_channel::{Receiver, Sender};
use dashmap::DashMap;
use hbbft::{crypto::{Signature, SignatureShare}, threshold_sign::ThresholdSign};
use indexmap::IndexMap;
use kademlia_dht::{Key, Node, NodeData};
use lr_trie::ReadHandleFactory;
use mempool::{mempool::{LeftRightMempool, TxnStatus}, Mempool, MempoolReadHandleFactory};
use patriecia::{db::MemoryDB, inner::InnerTrie};

use primitives::{
    FarmerQuorumThreshold,
    GroupPublicKey,
    HarvesterQuorumThreshold,
    NodeIdx,
    PeerId,
    QuorumType,
    RawSignature,
    TxHashString,
};
use rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use signer::signer::{SignatureProvider, Signer};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, Message, TheaterError};
use tokio::sync::{broadcast::error::TryRecvError, mpsc::UnboundedSender};
use tracing::error;
use vrrb_core::{
    bloom::Bloom,
    event_router::{DirectedEvent, Event, QuorumCertifiedTxn, Topic, Vote, VoteReceipt},
    txn::{TransactionDigest, Txn},
};

use crate::{
    result::Result,
    scheduler::{Job, JobResult},
    NodeError, validator_module::ValidatorModule, farmer_harvester_module::QuorumMember, harvester_module,
};


pub type QuorumId = String;
pub type QuorumPubkey = String;

#[allow(unused)]
pub struct Farmer {
    pub tx_mempool_reader: MempoolReadHandleFactory,
    pub group_public_key: GroupPublicKey,
    pub farmer_quorum_members: DashMap<QuorumId, Vec<PeerId>>,
    pub harvester_quorum_members: Vec<PeerId>,
    pub harvester_quorum_public_key: GroupPublicKey,
    pub sig_provider: SignatureProvider,
    pub farmer_id: PeerId,
    pub farmer_node_idx: NodeIdx,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    clear_filter_rx: tokio::sync::mpsc::UnboundedReceiver<DirectedEvent>,
    farmer_quorum_threshold: FarmerQuorumThreshold,
    harvester_quorum_threshold: HarvesterQuorumThreshold,
    sync_jobs_sender: Sender<Job>,
    async_jobs_sender: Sender<Job>,
    sync_jobs_status_receiver: Receiver<JobResult>,
    async_jobs_status_receiver: Receiver<JobResult>,
}


impl QuorumMember for Farmer {}


impl Farmer {
    pub fn new(
        tx_mempool_reader: MempoolReadHandleFactory,
        group_public_key: GroupPublicKey,
        farmer_quorum_members: DashMap<QuorumId, Vec<PeerId>>,
        harvester_quorum_members: Vec<PeerId>,
        harvester_quorum_public_key: GroupPublicKey,
        sig_provider: SignatureProvider,
        farmer_id: PeerId,
        farmer_node_idx: NodeIdx,
        status: ActorState,
        label: ActorLabel,
        id: ActorId,
        broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
        clear_filter_rx: tokio::sync::mpsc::UnboundedReceiver<DirectedEvent>,
        farmer_quorum_threshold: FarmerQuorumThreshold,
        harvester_quorum_threshold: HarvesterQuorumThreshold,
        sync_jobs_sender: Sender<Job>,
        async_jobs_sender: Sender<Job>,
        sync_jobs_status_receiver: Receiver<JobResult>,
        async_jobs_status_receiver: Receiver<JobResult>
    ) -> Self {

        Self {
            tx_mempool_reader,
            group_public_key,
            farmer_quorum_members,
            harvester_quorum_members,
            harvester_quorum_public_key,
            sig_provider,
            farmer_id,
            farmer_node_idx,
            status,
            label,
            id,
            broadcast_events_tx,
            clear_filter_rx,
            farmer_quorum_threshold,
            sync_jobs_sender,
            async_jobs_sender,
            sync_jobs_status_receiver,
            async_jobs_status_receiver
        }
    }

    pub fn name(&self) -> String {
        self.label.clone()
    }

    pub fn vote_valid(&self, digest: TransactionDigest) {
        let txn = self.tx_mempool_reader.get(&digest);
        if let Some(record) = txn {
            let payload = record.txn.build_payload();
            let result = self.sig_provider.generate_partial_signature(payload); 
            if let Ok(partial_sig) = result {

                let vote = Vote {
                    farmer_id: self.farmer_id.clone(),
                    farmer_node_id: self.farmer_node_idx.clone(),
                    signature: partial_sig,
                    txn: record.txn.clone(),
                    quorum_public_key: self.group_public_key.clone(),
                    quorum_threshold: self.farmer_quorum_threshold,
                    execution_result: None
                };

                self.broadcast_events_tx.send(
                    (Topic::Consensus, Event::Vote(vote)
                ); 
            }
        }
    }
}


#[async_trait]
impl Handler<Event> for Farmer {
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
            Event::ValidTxn(digest) => {
                // Send Vote to the Harvester Quorum:
                self.vote_valid(digest);
            },
            Event::PullQuorumCertifiedTxns(num_of_txns) => {
            },
            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}

