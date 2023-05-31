use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use block::{
    header::BlockHeader,
    valid::{BlockValidationData, Valid},
    Block,
    ConvergenceBlock,
    GenesisBlock,
    InnerBlock,
    ProposalBlock,
};
use bulldag::{
    graph::{BullDag, GraphError},
    vertex::Vertex,
};
use events::{Event, EventMessage, EventPublisher};
use hbbft::crypto::{PublicKeySet, Signature, SignatureShare, SIG_SIZE};
use primitives::SignatureType;
use signer::types::{SignerError, SignerResult};
use theater::{ActorId, ActorLabel, ActorState, Handler};
use tracing::{error, info};
use vrrb_core::claim::Claim;

pub type Edge = (Vertex<Block, String>, Vertex<Block, String>);
pub type Edges = Vec<Edge>;
pub type GraphResult<T> = Result<T, GraphError>;
///
/// The runtime module that manages the DAG, both exposing
/// data within and appending blocks to it.
///
/// ```
/// use std::sync::{Arc, RwLock};
///
/// use block::{header::BlockHeader, Block};
/// use bulldag::graph::BullDag;
/// use events::EventPublisher;
/// use hbbft::crypto::PublicKeySet;
/// use theater::{ActorId, ActorLabel, ActorState, Handler};
///
/// pub struct DagModule {
///     status: ActorState,
///     label: ActorLabel,
///     id: ActorId,
///     events_tx: EventPublisher,
///     dag: Arc<RwLock<BullDag<Block, String>>>,
///     public_key_set: Option<PublicKeySet>,
///     last_confirmed_block_header: Option<BlockHeader>,
/// }
/// ```
pub struct DagModule {
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    #[allow(unused)]
    events_tx: EventPublisher,
    dag: Arc<RwLock<BullDag<Block, String>>>,
    public_key_set: Option<PublicKeySet>,
    last_confirmed_block_header: Option<BlockHeader>,
    pub claim: Claim,
}

impl DagModule {
    pub fn new(
        dag: Arc<RwLock<BullDag<Block, String>>>,
        events_tx: EventPublisher,
        claim: Claim,
    ) -> Self {
        Self {
            status: ActorState::Stopped,
            label: String::from("Dag"),
            id: uuid::Uuid::new_v4().to_string(),
            events_tx,
            dag,
            public_key_set: None,
            last_confirmed_block_header: None,
            claim,
        }
    }

    pub fn set_harvester_pubkeys(&mut self, public_key_set: PublicKeySet) {
        self.public_key_set = Some(public_key_set);
    }

    pub fn append_genesis(&mut self, genesis: &GenesisBlock) -> GraphResult<()> {
        let valid = self.check_valid_genesis(genesis);

        if valid {
            let block: Block = genesis.clone().into();
            let vtx: Vertex<Block, String> = block.into();
            self.write_genesis(&vtx)?;
        }

        Ok(())
    }

    pub fn append_proposal(&mut self, proposal: &ProposalBlock) -> GraphResult<()> {
        let valid = self.check_valid_proposal(proposal);

        if valid {
            if let Ok(ref_block) = self.get_reference_block(&proposal.ref_block) {
                let block: Block = proposal.clone().into();
                let vtx: Vertex<Block, String> = block.into();
                let edge = (&ref_block, &vtx);
                self.write_edge(edge)?;
            } else {
                return Err(GraphError::NonExistentSource);
            }
        }

        Ok(())
    }

    pub fn append_convergence(&mut self, convergence: &ConvergenceBlock) -> GraphResult<()> {
        let valid = self.check_valid_convergence(convergence);
        if valid {
            let ref_blocks: Vec<Vertex<Block, String>> =
                self.get_convergence_reference_blocks(convergence);

            let block: Block = convergence.clone().into();
            let vtx: Vertex<Block, String> = block.into();
            let edges: Edges = ref_blocks
                .iter()
                .map(|ref_block| (ref_block.clone(), vtx.clone()))
                .collect();

            self.extend_edges(edges)?;
        }

        Ok(())
    }

    fn get_convergence_reference_blocks(
        &self,
        convergence: &ConvergenceBlock,
    ) -> Vec<Vertex<Block, String>> {
        convergence
            .get_ref_hashes()
            .iter()
            .filter_map(|target| match self.get_reference_block(target) {
                Ok(value) => Some(value),
                Err(_) => None,
            })
            .collect()
    }

    fn get_reference_block(&self, target: &String) -> GraphResult<Vertex<Block, String>> {
        if let Ok(guard) = self.dag.read() {
            if let Some(vtx) = guard.get_vertex(target.clone()) {
                return Ok(vtx.clone());
            }
        }

        Err(GraphError::NonExistentReference)
    }

    fn write_edge(
        &mut self,
        edge: (&Vertex<Block, String>, &Vertex<Block, String>),
    ) -> GraphResult<()> {
        if let Ok(mut guard) = self.dag.write() {
            guard.add_edge(edge);
            return Ok(());
        }

        Err(GraphError::Other("Error getting write guard".to_string()))
    }

    fn extend_edges(&mut self, edges: Edges) -> GraphResult<()> {
        let iter = edges.iter();

        for (ref_block, vtx) in iter {
            self.write_edge((ref_block, vtx))?
        }

        Ok(())
    }

    fn write_genesis(&self, vertex: &Vertex<Block, String>) -> GraphResult<()> {
        if let Ok(mut guard) = self.dag.write() {
            guard.add_vertex(vertex);

            return Ok(());
        }

        Err(GraphError::Other("Error getting write gurard".to_string()))
    }

    fn check_valid_genesis(&self, block: &GenesisBlock) -> bool {
        if let Ok(validation_data) = block.get_validation_data() {
            match self.verify_signature(validation_data) {
                Ok(true) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    fn check_valid_proposal(&self, block: &ProposalBlock) -> bool {
        if let Ok(validation_data) = block.get_validation_data() {
            match self.verify_signature(validation_data) {
                Ok(true) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    fn check_valid_convergence(&self, block: &ConvergenceBlock) -> bool {
        if let Ok(validation_data) = block.get_validation_data() {
            match self.verify_signature(validation_data) {
                Ok(true) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    fn verify_signature(&self, validation_data: BlockValidationData) -> SignerResult<bool> {
        if validation_data.signature.len() != SIG_SIZE {
            return Err(SignerError::CorruptSignatureShare(
                "Invalid Signature ,Size must be 96 bytes".to_string(),
            ));
        }
        match validation_data.signature_type.clone() {
            SignatureType::PartialSignature => self.verify_partial_sig(validation_data),
            SignatureType::ThresholdSignature | SignatureType::ChainLockSignature => {
                self.verify_threshold_sig(validation_data)
            },
        }
    }

    fn verify_partial_sig(&self, validation_data: BlockValidationData) -> SignerResult<bool> {
        let public_key_share = {
            if let Some(public_key_share) = self.public_key_set.clone() {
                if let Some(idx) = validation_data.node_idx {
                    public_key_share.public_key_share(idx as usize)
                } else {
                    return Err(SignerError::GroupPublicKeyMissing);
                }
            } else {
                return Err(SignerError::GroupPublicKeyMissing);
            }
        };

        if let Ok(signature_arr) = validation_data.signature.clone().try_into() {
            let signature_arr: [u8; 96] = signature_arr;

            match SignatureShare::from_bytes(signature_arr) {
                Ok(sig_share) => {
                    Ok(public_key_share.verify(&sig_share, validation_data.payload_hash))
                },
                Err(e) => Err(SignerError::SignatureVerificationError(format!(
                    "Error parsing partial signature details : {:?}",
                    e
                ))),
            }
        } else {
            Err(SignerError::PartialSignatureError(
                "Error parsing signature into array".to_string(),
            ))
        }
    }

    fn verify_threshold_sig(&self, validation_data: BlockValidationData) -> SignerResult<bool> {
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
                Ok(signature) => Ok(public_key_set
                    .public_key()
                    .verify(&signature, validation_data.payload_hash)),
                Err(e) => Err(SignerError::SignatureVerificationError(format!(
                    "Error parsing threshold signature details : {:?}",
                    e
                ))),
            }
        } else {
            Err(SignerError::PartialSignatureError(
                "Error parsing signature into array".to_string(),
            ))
        }
    }
}

#[async_trait]
impl Handler<EventMessage> for DagModule {
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

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            Event::BlockReceived(block) => match block {
                Block::Genesis { block } => {
                    if let Err(e) = self.append_genesis(&block) {
                        let err_note = format!("Encountered GraphError: {:?}", e);
                        return Err(theater::TheaterError::Other(err_note));
                    };
                },
                Block::Proposal { block } => {
                    if let Err(e) = self.append_proposal(&block) {
                        let err_note = format!("Encountered GraphError: {:?}", e);
                        return Err(theater::TheaterError::Other(err_note));
                    }
                },
                Block::Convergence { block } => {
                    if let Err(e) = self.append_convergence(&block) {
                        let err_note = format!("Encountered GraphError: {:?}", e);
                        return Err(theater::TheaterError::Other(err_note));
                    }
                    if block.certificate.is_none() {
                        if let Some(header) = self.last_confirmed_block_header.clone() {
                            self.events_tx
                                .send(EventMessage::new(
                                    None,
                                    Event::PrecheckConvergenceBlock(block.clone(), header.clone()),
                                ))
                                .await
                                .unwrap_or_else(|err| {
                                    error!(
                                    "Error occurred while broadcasting event {:?} ,details :{:?}",
                                    Event::PrecheckConvergenceBlock(block, header).to_string(),
                                    err
                                )
                                });
                        }
                    }
                },
            },
            Event::BlockCertificate(certificate) => {
                let mut mine_block = None;
                if let Ok(mut dag) = self.dag.write() {
                    if let Some(block) = dag.get_vertex_mut(certificate.block_hash.clone()) {
                        if let Block::Convergence { mut block } = block.get_data() {
                            block.append_certificate(certificate);
                            self.last_confirmed_block_header = Some(block.get_header());
                            mine_block = Some(block.clone());
                        }
                        // Emit event for state update
                    }
                }
                if let Some(block) = mine_block {
                    self.events_tx
                        .send(EventMessage::new(
                            None,
                            Event::MineProposalBlock(
                                block.hash.clone(),
                                block.get_header().round,
                                block.get_header().epoch,
                                self.claim.clone(),
                            ),
                        ))
                        .await
                        .unwrap_or_else(|err| {
                            error!(
                                "Error occurred while broadcasting event {:?} ,details :{:?}",
                                Event::MineProposalBlock(
                                    block.hash.clone(),
                                    block.get_header().round,
                                    block.get_header().epoch,
                                    self.claim.clone(),
                                )
                                .to_string(),
                                err
                            )
                        });
                }
            },
            Event::HarvesterPublicKey(pubkey_bytes) => {
                if let Ok(public_key_set) =
                    serde_json::from_slice::<PublicKeySet>(pubkey_bytes.as_slice())
                {
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
