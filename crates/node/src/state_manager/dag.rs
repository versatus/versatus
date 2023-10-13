use std::{
    collections::HashSet,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use block::{
    header::BlockHeader,
    valid::{BlockValidationData, Valid},
    Block, Certificate, ConvergenceBlock, GenesisBlock, InnerBlock, ProposalBlock,
};
use bulldag::{
    graph::{BullDag, GraphError},
    vertex::Vertex,
};
use hbbft::crypto::{PublicKeySet, PublicKeyShare, SignatureShare, SIG_SIZE};
use indexmap::IndexMap;
use primitives::{
    HarvesterQuorumThreshold, NodeId, PublicKey, QuorumType, RawSignature, Signature, SignatureType,
};
use signer::{
    engine::VALIDATION_THRESHOLD,
    types::{SignerError, SignerResult},
};
use signer::{
    engine::{QuorumData, QuorumMembers, SignerEngine},
    signer::Signer,
};
use theater::{ActorId, ActorState};
use vrrb_core::claim::Claim;

use crate::{NodeError, Result};

pub type Edge = (Vertex<Block, String>, Vertex<Block, String>);
pub type Edges = Vec<Edge>;
pub type GraphResult<T> = std::result::Result<T, GraphError>;

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
///     dag: Arc<RwLock<BullDag<Block, String>>>,
///     public_key_set: Option<PublicKeySet>,
///     last_confirmed_block_header: Option<BlockHeader>,
/// }
/// ```
#[derive(Clone, Debug)]
pub struct DagModule {
    dag: Arc<RwLock<BullDag<Block, String>>>,
    quorum_members: Option<QuorumMembers>,
    harvester_quorum_threshold: Option<HarvesterQuorumThreshold>,
    last_confirmed_block_header: Option<BlockHeader>,
    last_confirmed_block: Option<Block>,
    // String in next 2 fields represent the block hash
    pending_convergence_blocks: IndexMap<String, ConvergenceBlock>,
    pending_certificates: IndexMap<String, Certificate>,
    partial_certificate_signatures: IndexMap<String, HashSet<(NodeId, Signature)>>,
    // TODO: Why is the Claim here?
    claim: Claim,
}

impl DagModule {
    pub fn new(dag: Arc<RwLock<BullDag<Block, String>>>, claim: Claim) -> Self {
        Self {
            dag,
            quorum_members: None,
            harvester_quorum_threshold: None,
            last_confirmed_block_header: None,
            last_confirmed_block: None,
            pending_convergence_blocks: IndexMap::new(),
            pending_certificates: IndexMap::new(),
            partial_certificate_signatures: IndexMap::new(),
            claim,
        }
    }

    pub fn claim(&self) -> Claim {
        self.claim.clone()
    }

    pub fn read(&self) -> Result<RwLockReadGuard<BullDag<Block, String>>> {
        self.dag
            .read()
            .map_err(|err| NodeError::Other(err.to_string()))
    }

    pub fn dag(&self) -> Arc<RwLock<BullDag<Block, String>>> {
        self.dag.clone()
    }

    pub fn last_confirmed_block_header(&self) -> Option<BlockHeader> {
        self.last_confirmed_block_header.clone()
    }

    pub fn set_quorum_members(&mut self, quorum_members: QuorumMembers) {
        self.quorum_members = Some(quorum_members);
    }

    pub fn set_harvester_quorum_threshold(&mut self, quorum_threshold: HarvesterQuorumThreshold) {
        self.harvester_quorum_threshold = Some(quorum_threshold);
    }

    pub fn harvester_quorum_threshold(&self) -> Option<HarvesterQuorumThreshold> {
        self.harvester_quorum_threshold.clone()
    }

    pub fn get_pending_convergence_block(&self) -> Option<ConvergenceBlock> {
        todo!()
    }

    pub fn get_pending_convergence_block_mut(
        &mut self,
        key: &String,
    ) -> Option<&mut ConvergenceBlock> {
        self.pending_convergence_blocks.get_mut(key)
    }

    pub fn append_certificate_to_convergence_block(
        &mut self,
        certificate: &Certificate,
    ) -> GraphResult<Option<ConvergenceBlock>> {
        let mut block = self
            .get_pending_convergence_block_mut(&certificate.block_hash)
            .ok_or(GraphError::Other(
                "unable to find pending convergence block".to_string(),
            ))?
            .clone();

        block
            .append_certificate(certificate)
            .map_err(|err| GraphError::Other(err.to_string()))?;

        self.append_convergence(&mut block)
            .map_err(|err| GraphError::Other(format!("{:?}", err)))
    }

    pub fn append_genesis(&mut self, genesis: &GenesisBlock) -> GraphResult<()> {
        // TODO: re-enable checking genesis block certificates
        //
        // let valid = self.check_valid_genesis(genesis);
        // if !valid {
        //     return Err(GraphError::Other(format!(
        //         "invalid genesis block: {}",
        //         genesis.hash,
        //     )));
        // }

        // if valid {
        let block: Block = genesis.clone().into();
        let vtx: Vertex<Block, String> = block.clone().into();
        self.write_genesis(&vtx)?;

        self.last_confirmed_block_header = Some(genesis.header.clone());
        self.last_confirmed_block = Some(block);
        // }

        Ok(())
    }

    pub fn append_proposal(
        &mut self,
        proposal: &ProposalBlock,
        mut sig_engine: SignerEngine,
    ) -> GraphResult<()> {
        let valid = self.check_valid_proposal(proposal, sig_engine);

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

    pub fn append_convergence(
        &mut self,
        convergence: &ConvergenceBlock,
    ) -> GraphResult<Option<ConvergenceBlock>> {
        let valid = self.check_valid_convergence(convergence);

        if valid {
            let ref_blocks: Vec<Vertex<Block, String>> =
                self.get_convergence_reference_blocks(convergence);
            //dbg!(&ref_blocks);
            let block: Block = convergence.clone().into();
            let vtx: Vertex<Block, String> = block.into();
            //dbg!(&vtx);
            let edges: Edges = ref_blocks
                .iter()
                .map(|ref_block| (ref_block.clone(), vtx.clone()))
                .collect();
            self.extend_edges(edges)?;

            self.last_confirmed_block_header = Some(convergence.header.clone());
            self.last_confirmed_block = Some(Block::Convergence {
                block: convergence.clone(),
            });

            self.pending_convergence_blocks
                .remove(&convergence.hash)
                .ok_or(GraphError::Other(
                    "unable to find pending convergence block".to_string(),
                ))?;

            return Ok(Some(convergence.clone()));
        } else {
            self.pending_convergence_blocks
                .entry(convergence.hash.clone())
                .or_insert(convergence.clone());
        }

        Ok(None)
    }

    pub fn get_convergence_reference_blocks(
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

    fn get_reference_block(&self, target: &str) -> GraphResult<Vertex<Block, String>> {
        if let Ok(guard) = self.dag.read() {
            if let Some(vtx) = guard.get_vertex(target.to_owned()) {
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

    fn write_genesis(&mut self, vertex: &Vertex<Block, String>) -> GraphResult<()> {
        if let Ok(mut guard) = self.dag.write() {
            guard.add_vertex(vertex);

            return Ok(());
        }

        Err(GraphError::Other("Error getting write guard".to_string()))
    }

    //TODO: Move to test configured trait
    pub fn write_vertex(&mut self, vertex: &Vertex<Block, String>) -> GraphResult<()> {
        if let Ok(mut guard) = self.dag.write() {
            guard.add_vertex(vertex);

            return Ok(());
        }

        Err(GraphError::Other("Error getting write guard".to_string()))
    }

    fn check_valid_genesis(&self, block: &GenesisBlock, mut sig_engine: SignerEngine) -> bool {
        if let Ok(validation_data) = block.get_validation_data() {
            matches!(self.verify_signature(validation_data, sig_engine), Ok(true))
        } else {
            false
        }
    }

    fn check_valid_proposal(&self, block: &ProposalBlock, mut sig_engine: SignerEngine) -> bool {
        if let Ok(validation_data) = block.get_validation_data() {
            matches!(self.verify_signature(validation_data, sig_engine), Ok(true))
        } else {
            false
        }
    }

    //TODO: Refactor to return ConvergenceBlockStatus Enum as Pending
    // or Confirmed variant
    fn check_valid_convergence(&mut self, block: &ConvergenceBlock) -> bool {
        if let Some(_certificate) = &block.certificate {
            //TODO: Remove this as it is redundant...
            //match self.verify_certificate(certificate) {
            //Ok(true) => return true,
            //Ok(false) => return false,
            //Err(_) => return false,
            //}
            return true;
        }
        false
    }


    pub fn add_signer_to_convergence_block(
        &mut self,
        block_hash: String,
        sig: Signature,
        node_id: NodeId,
        sig_engine: &SignerEngine,
    ) -> Result<HashSet<(NodeId, Signature)>> {
        match self
            .partial_certificate_signatures
            .entry(block_hash.clone())
        {
            indexmap::map::Entry::Occupied(mut entry) => {
                entry.get_mut().insert((node_id, sig.clone()));
            },
            indexmap::map::Entry::Vacant(entry) => {
                let mut set = HashSet::new();
                set.insert((node_id, sig.clone()));
                entry.insert(set);
            },
        }
        self.check_certificate_threshold_reached(&block_hash, sig_engine)
    }

    pub fn check_certificate_threshold_reached(
        &self,
        block_hash: &String,
        sig_engine: &SignerEngine,
    ) -> Result<HashSet<(NodeId, Signature)>> {
        if let Some(set) = self.partial_certificate_signatures.get(block_hash) {
            if &set.len() >= &sig_engine.quorum_members().get_harvester_threshold() {
                return Ok(set.clone());
            }
        }

        Err(NodeError::Other(format!("threshold not reached")))
    }

    fn check_invalid_partial_sig(&self, block_hash: String) -> SignerResult<NodeId> {
        todo!()
    }

    // This is probably redundant
    fn verify_certificate(&self, certificate: &Certificate) -> SignerResult<bool> {
        todo!();
    }

    fn verify_certificate_signature(
        &self,
        signature: &mut Vec<(NodeId, Signature)>,
        sig_type: SignatureType,
        payload_hash: Vec<u8>,
        mut sig_engine: SignerEngine,
    ) -> SignerResult<(bool, SignatureType)> {
        match sig_type {
            SignatureType::PartialSignature => {
                if let Some((id, sig)) = signature.pop() {
                    self.verify_certificate_partial_sig(sig, id, payload_hash, sig_engine)
                } else {
                    Err(SignerError::PartialSignatureError(
                        "no signature provided".to_string(),
                    ))
                }
            },
            SignatureType::ThresholdSignature | SignatureType::ChainLockSignature => {
                self.verify_certificate_threshold_sig(signature.clone(), payload_hash, sig_engine)
            },
        }
    }

    fn verify_certificate_partial_sig(
        &self,
        sig: Signature,
        node_idx: NodeId,
        payload_hash: Vec<u8>,
        mut sig_engine: SignerEngine,
    ) -> SignerResult<(bool, SignatureType)> {
        let public_keyshare = self.get_harvester_public_keyshare(node_idx)?;
        self.verify_partial_sig_with_public_keyshare(sig, public_keyshare, payload_hash, sig_engine)
    }

    fn verify_certificate_threshold_sig(
        &self,
        sigs: Vec<(NodeId, Signature)>,
        payload_hash: Vec<u8>,
        mut sig_engine: SignerEngine,
    ) -> SignerResult<(bool, SignatureType)> {
        self.verify_threshold_sig_with_public_keyset(sigs, payload_hash, sig_engine)
    }

    fn verify_threshold_sig_with_public_keyset(
        &self,
        sigs: Vec<(NodeId, Signature)>,
        payload_hash: Vec<u8>,
        mut sig_engine: SignerEngine,
    ) -> SignerResult<(bool, SignatureType)> {
        sig_engine
            .verify_batch(&sigs, &payload_hash)
            .map_err(|err| SignerError::ThresholdSignatureError(err.to_string()))?;

        Ok((true, SignatureType::ThresholdSignature))
    }

    fn verify_partial_sig_with_public_keyshare(
        &self,
        sig: Signature,
        public_keyshare: PublicKey,
        payload_hash: Vec<u8>,
        mut sig_engine: SignerEngine,
    ) -> SignerResult<(bool, SignatureType)> {
        if let Some(mut harvesters) = sig_engine.quorum_members().get_harvester_data() {
            harvesters
                .members
                .retain(|id, pk| pk.clone() == public_keyshare);
            if let Some((id, pk)) = harvesters.members.iter().next() {
                sig_engine.verify(id, &sig, &payload_hash).map_err(|err| {
                    SignerError::PartialSignatureError(format!("unable to verify signature"))
                })?;
                Ok((true, SignatureType::PartialSignature))
            } else {
                return Err(SignerError::PartialSignatureError(
                    "unable to find signer in sig engine".to_string(),
                ));
            }
        } else {
            return Err(SignerError::PartialSignatureError(
                "Error parsing signature into array".to_string(),
            ));
        }
    }

    #[deprecated]
    fn verify_signature(
        &self,
        validation_data: BlockValidationData,
        mut sig_engine: SignerEngine,
    ) -> SignerResult<bool> {
        todo!()
    }

    #[deprecated]
    fn verify_partial_sig(
        &self,
        validation_data: BlockValidationData,
        mut sig_engine: SignerEngine,
    ) -> SignerResult<bool> {
        todo!();
    }

    #[deprecated]
    fn verify_threshold_sig(
        &self,
        validation_data: BlockValidationData,
        mut sig_engine: SignerEngine,
    ) -> SignerResult<bool> {
        let sig_set = validation_data.signatures.clone();
        if sig_set.len() <= sig_engine.quorum_members().get_harvester_threshold() {
            return Err(SignerError::ThresholdSignatureError(format!(
                "not enough signatures received to meet threshold"
            )));
        }

        sig_engine
            .verify_batch(&sig_set, &validation_data.payload_hash)
            .map_err(|err| SignerError::ThresholdSignatureError(err.to_string()))?;

        Ok(true)
    }

    fn get_harvester_public_keyshare(&self, node_id: NodeId) -> SignerResult<PublicKey> {
        let public_key_share = {
            if let Some(quorum_members) = self.quorum_members.clone() {
                if let Some(data) = quorum_members.get_harvester_data() {
                    if let Some(key) = data.members.get(&node_id) {
                        key.clone()
                    } else {
                        return Err(SignerError::GroupPublicKeyMissing);
                    }
                } else {
                    return Err(SignerError::GroupPublicKeyMissing);
                }
            } else {
                return Err(SignerError::GroupPublicKeyMissing);
            }
        };

        Ok(public_key_share)
    }

    fn get_harvester_public_keyset(&self) -> SignerResult<Vec<PublicKey>> {
        let public_keyset = {
            if let Some(quorum_members) = self.quorum_members.clone() {
                if let Some(data) = quorum_members.get_harvester_data() {
                    data.members.values().cloned().into_iter().collect()
                } else {
                    return Err(SignerError::GroupPublicKeyMissing);
                }
            } else {
                return Err(SignerError::GroupPublicKeyMissing);
            }
        };

        Ok(public_keyset)
    }

    fn get_harvester_node_ids(&self) -> SignerResult<Vec<NodeId>> {
        let node_ids = {
            if let Some(quorum_members) = self.quorum_members.clone() {
                if let Some(data) = quorum_members.get_harvester_data() {
                    data.members.keys().cloned().into_iter().collect()
                } else {
                    return Err(SignerError::GroupPublicKeyMissing);
                }
            } else {
                return Err(SignerError::GroupPublicKeyMissing);
            }
        };

        Ok(node_ids)
    }
}
