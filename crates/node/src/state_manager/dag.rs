use std::{sync::{Arc, RwLock, RwLockReadGuard}, collections::HashSet};

use block::{
    header::BlockHeader,
    valid::{BlockValidationData, Valid},
    Block, ConvergenceBlock, GenesisBlock, InnerBlock, ProposalBlock, QuorumMembers, Certificate,
};
use bulldag::{
    graph::{BullDag, GraphError},
    vertex::Vertex,
};
use hbbft::crypto::{PublicKeySet, PublicKeyShare, Signature, SignatureShare, SIG_SIZE};
use indexmap::IndexMap;
use primitives::{SignatureType, QuorumType, RawSignature, HarvesterQuorumThreshold, NodeId};
use signer::types::{SignerError, SignerResult};
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
    partial_certificate_signatures: IndexMap<String, HashSet<(NodeId, RawSignature)>>,
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

    pub fn append_convergence(&mut self, convergence: &mut ConvergenceBlock) -> GraphResult<()> {
        let valid = self.check_valid_convergence(convergence);



        // TODO: Can we remove the commented out code below?
        // if !valid {
        //     return Err(GraphError::Other(format!(
        //         "invalid convergence block: {}",
        //         convergence.hash,
        //     )));
        // }


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

            self.last_confirmed_block_header = Some(convergence.header.clone());
            self.last_confirmed_block = Some(Block::Convergence {
                block: convergence.to_owned(),
            });
        } else {
            self.pending_convergence_blocks
                .entry(convergence.hash.clone())
                .or_insert(convergence.clone());
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

    fn check_valid_genesis(&self, block: &GenesisBlock) -> bool {
        if let Ok(validation_data) = block.get_validation_data() {
            matches!(self.verify_signature(validation_data), Ok(true))
        } else {
            false
        }
    }

    fn check_valid_proposal(&self, block: &ProposalBlock) -> bool {
        if let Ok(validation_data) = block.get_validation_data() {
            matches!(self.verify_signature(validation_data), Ok(true))
        } else {
            false
        }
    }

    //TODO: Refactor to return ConvergenceBlockStatus Enum as Pending
    // or Confirmed variant
    fn check_valid_convergence(&mut self, block: &mut ConvergenceBlock) -> bool {
        if let Some(certificate) = &block.certificate {
            match self.verify_certificate(certificate) {
                Ok(true) => return true,
                Ok(false) => return false,
                Err(_) => return false
            }
        } 
        false 
    }

    pub fn add_signer_to_convergence_block(
        &mut self, 
        block_hash: String, 
        sig: RawSignature, 
        node_id: NodeId,
    ) -> Result<HashSet<(NodeId, RawSignature)>> {
        match self.partial_certificate_signatures.entry(block_hash.clone()) {
            indexmap::map::Entry::Occupied(mut entry) => {
                entry.get_mut().insert((node_id, sig.clone()));
            },
            indexmap::map::Entry::Vacant(entry) => {
                let mut set = HashSet::new();
                set.insert((node_id, sig.clone()));
                entry.insert(set);
            }
        }
        self.check_threshold_reached(&block_hash) 
    }

    fn check_threshold_reached(&self, block_hash: &String) -> Result<HashSet<(NodeId, RawSignature)>> {
        if let Some(set) = self.partial_certificate_signatures.get(block_hash) {
            if let Some(threshold) = &self.harvester_quorum_threshold {
                if &set.len() >= threshold {
                    return Ok(set.clone())
                }
            }
        }

        Err(NodeError::Other(format!("threshold not reached")))
    }

    pub fn form_convergence_certificate(&self, block_hash: String, sig: RawSignature) -> Result<Certificate> {
//        let block_hash = block.hash.clone();
//        if let Some(sigs) = self.partial_certificate_signatures.get(&block_hash) {
//            if let Some(threshold) = &self.harvester_quorum_threshold {
//                if &sigs.len() >= threshold {
//                    return Ok(())
//                }
//            }
//        }

//        Ok(())
        todo!()
    }

    fn check_invalid_partial_sig(&self, block_hash: String) -> SignerResult<NodeId> {
        todo!()
    }

    fn verify_certificate(&self, certificate: &Certificate) -> SignerResult<bool> {
        todo!();
    }

    fn verify_certificate_signature(
        &self, 
        signature: RawSignature, 
        sig_type: SignatureType,
        node_idx: Option<u16>,
        payload_hash: Vec<u8>,
    ) -> SignerResult<(bool, SignatureType)> {
        if signature.len() != SIG_SIZE {
            return Err(SignerError::CorruptSignatureShare(
                    "invalid signature, size must be 96 bytes".to_string(),
            ));
        }

        match sig_type {
            SignatureType::PartialSignature => {
                self.verify_certificate_partial_sig(
                    signature, 
                    node_idx,
                    payload_hash
                )
            },
            SignatureType::ThresholdSignature | SignatureType::ChainLockSignature => {
                self.verify_certificate_threshold_sig(
                    signature,
                    payload_hash
                )
            }
        }
    }

    fn verify_certificate_partial_sig(
        &self, 
        sig: RawSignature, 
        node_idx: Option<u16>,
        payload_hash: Vec<u8>
    ) -> SignerResult<(bool, SignatureType)> {
        let public_keyshare = self.get_harvester_public_keyshare(node_idx)?;
        self.verify_partial_sig_with_public_keyshare(
            sig, 
            public_keyshare, 
            payload_hash
        )
    }

    fn verify_certificate_threshold_sig(
        &self, 
        sig: RawSignature,
        payload_hash: Vec<u8>,
    ) -> SignerResult<(bool, SignatureType)> {
        let public_keyset = self.get_harvester_public_keyset()?;
        self.verify_threshold_sig_with_public_keyset(sig, public_keyset, payload_hash)
    }

    fn verify_threshold_sig_with_public_keyset(
        &self,
        sig: RawSignature,
        public_keyset: PublicKeySet,
        payload_hash: Vec<u8>,
    ) -> SignerResult<(bool, SignatureType)> {
        if let Ok(signature_arr) = sig.clone().try_into() {
            let signature_arr: [u8; 96] = signature_arr;
            match Signature::from_bytes(signature_arr) {
                Ok(signature) => {
                    Ok((
                    public_keyset.public_key().verify(&signature, payload_hash),
                    SignatureType::ThresholdSignature
                ))
                },
                Err(e) => Err(SignerError::SignatureVerificationError(format!(
                    "Error parsing threshold signature details: {:?}",
                    e
                ))),
            }
        } else {
            Err(SignerError::PartialSignatureError(
                "Error parsing signature into array".to_string(),
            ))
        }
    }

    fn verify_partial_sig_with_public_keyshare(
        &self, 
        sig: RawSignature,
        public_keyshare: PublicKeyShare, 
        payload_hash: Vec<u8>
    ) -> SignerResult<(bool, SignatureType)> {
        if let Ok(signature_arr) = sig.clone().try_into() {
            let signature_arr: [u8; 96] = signature_arr;

            match SignatureShare::from_bytes(signature_arr) {
                Ok(sig_share) => {
                    return Ok((
                        public_keyshare.verify(&sig_share, payload_hash), 
                        SignatureType::PartialSignature
                    ))
                },
                Err(e) => {
                    return Err(SignerError::SignatureVerificationError(format!(
                    "Error parsing partial signature details : {:?}",
                    e
                )))},
            }
        } else {
            return Err(SignerError::PartialSignatureError(
                "Error parsing signature into array".to_string(),
            ));
        }
    }

    fn verify_signature(&self, validation_data: BlockValidationData) -> SignerResult<bool> {
        if validation_data.signature.len() != SIG_SIZE {
            return Err(SignerError::CorruptSignatureShare(
                "Invalid Signature, size must be 96 bytes".to_string(),
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
            if let Some(quorum_members) = self.quorum_members.clone() {
                if let Some(public_key_set) = quorum_members.values().find(|quorum_data| {
                    quorum_data.quorum_type == QuorumType::Harvester
                }).map(|quorum_data| quorum_data.quorum_pubkey.clone()) {
                    if let Some(idx) = validation_data.node_idx {
                        public_key_set.public_key_share(idx as usize)
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
            if let Some(quorum_members) = self.quorum_members.clone() {
                if let Some(public_key_set) = quorum_members.values().find(|quorum_data| {
                    quorum_data.quorum_type == QuorumType::Harvester
                }).map(|quorum_data| quorum_data.quorum_pubkey.clone()) {
                    public_key_set
                } else {
                    return Err(SignerError::GroupPublicKeyMissing);
                }
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
                    "Error parsing threshold signature details: {:?}",
                    e
                ))),
            }
        } else {
            Err(SignerError::PartialSignatureError(
                "Error parsing signature into array".to_string(),
            ))
        }
    }

    fn get_harvester_public_keyshare(&self, node_idx: Option<u16>) -> SignerResult<PublicKeyShare> {
        let public_key_share = {
            if let Some(quorum_members) = self.quorum_members.clone() {
                if let Some(public_key_set) = quorum_members.values().find(|quorum_data| {
                    quorum_data.quorum_type == QuorumType::Harvester
                }).map(|quorum_data| quorum_data.quorum_pubkey.clone()) {
                    if let Some(idx) = node_idx {
                        public_key_set.public_key_share(idx as usize)
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

    fn get_harvester_public_keyset(&self) -> SignerResult<PublicKeySet> {
        let public_keyset = {
            if let Some(quorum_members) = self.quorum_members.clone() {
                if let Some(public_key_set) = quorum_members.values().find(|quorum_data| {
                    quorum_data.quorum_type == QuorumType::Harvester
                }).map(|quorum_data| quorum_data.quorum_pubkey.clone()) {
                    public_key_set
                } else {
                    return Err(SignerError::GroupPublicKeyMissing);
                }
            } else {
                return Err(SignerError::GroupPublicKeyMissing);
            }
        };

        Ok(public_keyset)

    }
}
