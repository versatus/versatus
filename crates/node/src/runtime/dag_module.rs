use std::{sync::{Arc, RwLock}, collections::BTreeMap};

use block::{Block, ProposalBlock, ConvergenceBlock, InnerBlock, GenesisBlock};
use bulldag::{graph::{BullDag, GraphError}, vertex::Vertex};
use hbbft::crypto::{PublicKeySet, PublicKey, SIG_SIZE, SignatureShare, Signature};
use primitives::{SignatureType, RawSignature, PayloadHash, NodeIdx};
use signer::types::{SignerError, SignerResult};
use dkg_engine::types::NodeID;
use theater::{ActorState, ActorLabel, ActorId, Handler};
use async_trait::async_trait;
use events::Event;
use telemetry::info;

use crate::EventBroadcastSender;

pub type Edge = (Vertex<Block, String>, Vertex<Block, String>);
pub type Edges = Vec<Edge>;
pub type GraphResult<T> = Result<T, GraphError>;


/// The runtime module that manages the DAG, both exposing 
/// data within and appending blocks to it.
///
/// ```
/// use std::sync::{Arc, RwLock};
///
/// use block::Block;
/// use bulldag::graph::BullDag;
/// use node::EventBroadcastSender;
/// use theater::{ActorState, ActorLabel, ActorId, Handler};
///
/// pub struct DagModule {
///     status: ActorState,
///     label: ActorLabel,
///     id: ActorId,
///     events_tx: EventBroadcastSender,
///     dag: Arc<RwLock<BullDag<Block, String>>>,
/// }
/// ```
pub struct DagModule {
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: EventBroadcastSender,
    dag: Arc<RwLock<BullDag<Block, String>>>,
    peer_public_keys: BTreeMap<NodeID, PublicKey>,
    public_key_set: Option<PublicKeySet>,
    //harvester_pubkeys
    //harvester_quorum_threshold_pubkey
}

impl DagModule {
    pub fn new(
        dag: Arc<RwLock<BullDag<Block, String>>>,
        events_tx: EventBroadcastSender
    ) -> Self {
        let peer_public_keys: BTreeMap<NodeID, PublicKey> = BTreeMap::new();
        Self {
            status: ActorState::Stopped,
            label: String::from("Dag"),
            id: uuid::Uuid::new_v4().to_string(),
            events_tx,
            dag,
            peer_public_keys,
            public_key_set: None,
        }
    }

    pub fn set_harvester_pubkeys(
        &mut self, 
        public_key_set: PublicKeySet,
        peer_public_keys: BTreeMap<NodeID, PublicKey>,
    ) {
        self.peer_public_keys = peer_public_keys;
        self.public_key_set = Some(public_key_set);
    }

    pub fn append_genesis(
        &mut self,
        genesis: &GenesisBlock
    ) -> GraphResult<()> {
        let block: Block = genesis.clone().into();
        let vtx: Vertex<Block, String> = block.into();
        self.write_genesis(&vtx)?;
        
        Ok(())
    }

    pub fn append_proposal(
        &mut self, 
        proposal: &ProposalBlock) -> GraphResult<()> {
        if let Ok(ref_block) = self.get_reference_block(
            &proposal.ref_block
        ) {
            let block: Block = proposal.clone().into();
            let vtx: Vertex<Block, String> = block.into();
            let edge = (&ref_block, &vtx);
            self.write_edge(edge)?; 
        }; 


        Ok(())
    }

    pub fn append_convergence(
        &mut self,
        convergence: &ConvergenceBlock
    ) -> GraphResult<()> {

        let ref_blocks: Vec<Vertex<Block, String>> = self
            .get_convergence_reference_blocks(
                convergence
        );

        let block: Block = convergence.clone().into();
        let vtx: Vertex<Block, String> = block.into();
        let edges: Edges = ref_blocks.iter().map(|ref_block| {
            (ref_block.clone(), vtx.clone())
        }).collect();

        self.extend_edges(edges)?;

        Ok(())
    }

    fn get_convergence_reference_blocks(
        &self, convergence: &ConvergenceBlock
    ) -> Vec<Vertex<Block, String>> {
        convergence
            .get_ref_hashes()
            .iter()
            .filter_map(|target| {
                match self.get_reference_block(target) {
                    Ok(value) => Some(value),
                    Err(_) => None,
                }
            }).collect()
    }

    fn get_reference_block(
        &self, 
        target: &String
    ) -> GraphResult<Vertex<Block, String>> {

        if let Ok(guard) = self.dag.read() {
            if let Some(vtx) = guard.get_vertex(target.clone()) {
                return Ok(vtx.clone())
            }
        }

        return Err(GraphError::NonExistentReference)
    }

    fn write_edge(
        &mut self, 
        edge: (&Vertex<Block, String>, &Vertex<Block, String>)
    ) -> GraphResult<()> {
        if let Ok(mut guard) = self.dag.write() {
            guard.add_edge(edge);
            return Ok(())
        }

        return Err(GraphError::Other("Error getting write guard".to_string()));
    }

    fn extend_edges(
        &mut self,
        edges: Edges
    ) -> GraphResult<()> {
        let mut iter = edges.iter();
        
        while let Some((ref_block, vtx)) = iter.next() {
            if let Err(e) = self.write_edge((ref_block, vtx)) {
                return Err(e)
            }
        }

        Ok(())
    }

    fn write_genesis(
        &self,
        vertex: &Vertex<Block, String>
    ) -> GraphResult<()> {

        if let Ok(mut guard) = self.dag.write() {
            guard.add_vertex(vertex);

            return Ok(());
        }

        return Err(GraphError::Other("Error getting write gurard".to_string()));
    }

    fn verify_signature(
        &self,
        node_idx: NodeIdx,
        payload_hash: PayloadHash,
        signature: RawSignature,
        signature_type: SignatureType,
    ) -> SignerResult<bool> {
        if signature.len() != SIG_SIZE {
            return Err(SignerError::CorruptSignatureShare(
                "Invalid Signature ,Size must be 96 bytes".to_string(),
            ));
        }
        match signature_type {
            SignatureType::PartialSignature => {
                self.verify_partial_sig(node_idx, payload_hash, signature)
            },
            SignatureType::ThresholdSignature | SignatureType::ChainLockSignature => {
                self.verify_threshold_sig(payload_hash, signature)
            }
        }
    }

    fn verify_partial_sig(
        &self,
        node_idx: NodeIdx,
        payload_hash: PayloadHash,
        signature: RawSignature 
    ) -> SignerResult<bool> {
        let public_key_share = {
            if let Some(public_key_share) = self.public_key_set.clone() {
                public_key_share.public_key_share(node_idx as usize)
            } else {
                return Err(SignerError::GroupPublicKeyMissing)
            }
        };

        let signature_arr: [u8; 96] = signature.try_into().unwrap();
        match SignatureShare::from_bytes(signature_arr) {
            Ok(sig_share) => {
                return Ok(public_key_share.verify(&sig_share, payload_hash))
            },
            Err(e) => {
                return Err(SignerError::SignatureVerificationError(format!(
                    "Error parsing partial signature details : {:?}",
                    e
                )))
            },
        }
    }

    fn verify_threshold_sig(
        &self,
        payload_hash: PayloadHash,
        signature: RawSignature
    ) -> SignerResult<bool> {

        let public_key_set = {
            if let Some(public_key_set) = self.public_key_set.clone() {
                public_key_set
            } else {
                return Err(SignerError::GroupPublicKeyMissing);
            }
        };
        let signature_arr: [u8; 96] = signature.try_into().unwrap();
        match Signature::from_bytes(signature_arr) {
            Ok(signature) => {
                return Ok(
                    public_key_set
                        .public_key()
                        .verify(&signature, payload_hash)
                )
            },
            Err(e) => {
                return Err(SignerError::SignatureVerificationError(format!(
                    "Error parsing threshold signature details : {:?}",
                    e
                )))
            }
        }
    }
}


#[async_trait]
impl Handler<Event> for DagModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.label.clone()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_start(&self) {
        info!("{}-{} starting", self.label(), self.id(),);
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.label(),
            self.id(),
        );
    }

    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            Event::BlockReceived(block) => {
                match block {
                    Block::Genesis { block } => {
                        self.append_genesis(&block);
                    },
                    Block::Proposal { block } => {
                        if let Err(e) = self.append_proposal(&block) {
                            let err_note = format!(
                                "Encountered GraphError: {:?}", e
                            );
                            return Err(theater::TheaterError::Other(err_note));
                        }
                    },
                    Block::Convergence { block } => {
                        if let Err(e) = self.append_convergence(&block) {
                            let err_note = format!(
                                "Encountered GraphError: {:?}", e
                            );
                            return Err(theater::TheaterError::Other(err_note));
                        }
                    }
                }
            },
            Event::NoOp => {},
            // _ => telemetry::warn!("unrecognized command received: {:?}", event),
            _ => {},
        }
        Ok(ActorState::Running)
    }
}
