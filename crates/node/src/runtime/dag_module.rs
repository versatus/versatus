use std::sync::{Arc, RwLock};

use block::{Block, ProposalBlock, ConvergenceBlock, InnerBlock, GenesisBlock, valid::{BlockValidationData, Valid}};
use bulldag::{graph::{BullDag, GraphError}, vertex::Vertex};
use hbbft::crypto::{PublicKeySet, SIG_SIZE, SignatureShare, Signature};
use primitives::SignatureType;
use signer::types::{SignerError, SignerResult};
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
/// use hbbft::crypto::PublicKeySet;
///
/// pub struct DagModule {
///     status: ActorState,
///     label: ActorLabel,
///     id: ActorId,
///     events_tx: EventBroadcastSender,
///     dag: Arc<RwLock<BullDag<Block, String>>>,
///     public_key_set: Option<PublicKeySet>
/// }
/// ```
pub struct DagModule {
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    #[allow(unused)]
    events_tx: EventBroadcastSender,
    dag: Arc<RwLock<BullDag<Block, String>>>,
    public_key_set: Option<PublicKeySet>,
}

impl DagModule {
    pub fn new(
        dag: Arc<RwLock<BullDag<Block, String>>>,
        events_tx: EventBroadcastSender
    ) -> Self {
        Self {
            status: ActorState::Stopped,
            label: String::from("Dag"),
            id: uuid::Uuid::new_v4().to_string(),
            events_tx,
            dag,
            public_key_set: None,
        }
    }

    pub fn set_harvester_pubkeys(
        &mut self, 
        public_key_set: PublicKeySet,
    ) {
        self.public_key_set = Some(public_key_set);
    }

    pub fn append_genesis(
        &mut self,
        genesis: &GenesisBlock
    ) -> GraphResult<()> {
        let valid = self.check_valid_genesis(genesis); 

        if valid {
            let block: Block = genesis.clone().into();
            let vtx: Vertex<Block, String> = block.into();
            self.write_genesis(&vtx)?;
        }
        
        return Ok(())
    }

    pub fn append_proposal(
        &mut self, 
        proposal: &ProposalBlock) -> GraphResult<()> {
       
        let valid = self.check_valid_proposal(proposal);
        
        if valid {
            if let Ok(ref_block) = self.get_reference_block(
                &proposal.ref_block
            ) {
                let block: Block = proposal.clone().into();
                let vtx: Vertex<Block, String> = block.into();
                let edge = (&ref_block, &vtx);
                self.write_edge(edge)?; 
            } else { 
                return Err(GraphError::NonExistentSource)
            }
        }

        Ok(())
    }

    pub fn append_convergence(
        &mut self,
        convergence: &ConvergenceBlock
    ) -> GraphResult<()> {

        let valid = self.check_valid_convergence(convergence);

        if valid {
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
        }
        
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

    fn check_valid_genesis(&self, block: &GenesisBlock) -> bool {
        if let Ok(validation_data) = block.get_validation_data() {
            match self.verify_signature(validation_data) {
                Ok(true) => return true,
                _ => return false
            }
        } else {
            return false
        }
    }

    fn check_valid_proposal(&self, block: &ProposalBlock) -> bool {
        if let Ok(validation_data) = block.get_validation_data() {
            match self.verify_signature(validation_data) {
                Ok(true) => return true,
                _ => return false
            }
        } else {
            return false
        }
    }

    fn check_valid_convergence(&self, block: &ConvergenceBlock) -> bool {
        if let Ok(validation_data) = block.get_validation_data() {
            match self.verify_signature(validation_data) {
                Ok(true) => return true,
                _ => return false
            }
        } else {
            return false
        }
    }

    fn verify_signature(
        &self,
        validation_data: BlockValidationData
    ) -> SignerResult<bool> {
        if validation_data.signature.clone().len() != SIG_SIZE {
            return Err(SignerError::CorruptSignatureShare(
                "Invalid Signature ,Size must be 96 bytes".to_string(),
            ));
        }
        match validation_data.signature_type.clone() {
            SignatureType::PartialSignature => {
                return self.verify_partial_sig(validation_data);
            },
            SignatureType::ThresholdSignature | SignatureType::ChainLockSignature => {
                return self.verify_threshold_sig(validation_data);
            }
        }
    }

    fn verify_partial_sig(
        &self,
        validation_data: BlockValidationData
    ) -> SignerResult<bool> {
        let public_key_share = {
            if let Some(public_key_share) = self.public_key_set.clone() {
                if let Some(idx) = validation_data.node_idx.clone() {
                    public_key_share.public_key_share(idx as usize)
                } else {
                    return Err(SignerError::GroupPublicKeyMissing)
                }
            } else {
                return Err(SignerError::GroupPublicKeyMissing)
            }
        };

        if let Ok(signature_arr) = validation_data.signature.clone().try_into() {
            let signature_arr: [u8; 96] = signature_arr;

            match SignatureShare::from_bytes(signature_arr) {
                Ok(sig_share) => {
                    return Ok(
                        public_key_share.verify(
                            &sig_share, 
                            validation_data.payload_hash.clone()
                        )
                    )
                },
                Err(e) => {
                    return Err(SignerError::SignatureVerificationError(format!(
                        "Error parsing partial signature details : {:?}",
                        e
                    )))
                },
            }
        } else {
            return Err(SignerError::PartialSignatureError(
                format!(
                    "Error parsing signature into array"
                )
            ))
        }
    }

    fn verify_threshold_sig(
        &self,
        validation_data: BlockValidationData
    ) -> SignerResult<bool> {

        let public_key_set = {
            if let Some(public_key_set) = self.public_key_set.clone() {
                public_key_set
            } else {
                return Err(SignerError::GroupPublicKeyMissing);
            }
        };

        if let Ok(signature_arr) = validation_data.signature.clone().try_into() {
            let signature_arr: [u8; 96] = signature_arr;
            match Signature::from_bytes(signature_arr) {
                Ok(signature) => {
                    return Ok(
                        public_key_set
                            .public_key()
                            .verify(&signature, validation_data.payload_hash)
                    )
                },
                Err(e) => {
                    return Err(SignerError::SignatureVerificationError(format!(
                        "Error parsing threshold signature details : {:?}",
                        e
                    )))
                }
            }
        } else {
            return Err(SignerError::PartialSignatureError(
                format!(
                    "Error parsing signature into array"
                )
            ))
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
                        if let Err(e) = self.append_genesis(&block) {
                            let err_note = format!(
                                "Encountered GraphError: {:?}", e 
                            );
                            return Err(theater::TheaterError::Other(err_note));
                        };
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
            Event::HarvesterPublicKey(pubkey_bytes) => {
                if let Ok(public_key_set) = 
                    serde_json::from_slice::<PublicKeySet>(&pubkey_bytes) {
                    self.set_harvester_pubkeys(public_key_set)
                }
            },
            Event::NoOp => {},
            // _ => telemetry::warn!("unrecognized command received: {:?}", event),
            _ => {},
        }
        Ok(ActorState::Running)
    }
}
