use primitives::{NodeId, PublicKey, QuorumId, SecretKey, Signature, QuorumType};
use secp256k1::Message;
use sha2::{Digest, Sha256};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::hash::Hasher;
use std::cmp::Ord;

pub const VALIDATION_THRESHOLD: f64 = 0.6;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[repr(C)]
pub struct QuorumData {
    pub id: QuorumId,
    pub quorum_type: QuorumType,
    pub members: HashMap<NodeId, PublicKey>,
}

impl std::hash::Hash for QuorumData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.quorum_type.hash(state);
        let members: Vec<(NodeId, PublicKey)> = self.members.clone().into_iter().collect();
        members.hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct QuorumMembers(pub HashMap<QuorumId, QuorumData>);
impl std::hash::Hash for QuorumMembers {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Get a mutable reference to the inner HashMap
        let map = &self.0;

        // Collect the entries into a Vec and sort them by key
        let mut entries: Vec<_> = map.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));

        // Hash each entry in order
        for (key, value) in entries {
            key.hash(state);
            value.hash(state);
        }
    }
}

impl QuorumMembers {
    pub fn get_public_key_from_members(&self, k: &NodeId) -> Option<PublicKey> {
        for (_, quorum_data) in self.0.iter() {
            if let Some(pub_key) = quorum_data.members.get(k) {
                return Some(pub_key.clone());
            }
        }
        None
    }

    pub fn get_harvester_data(&self) -> Option<QuorumData> {
        for (_, quorum_data) in self.0.iter() {
            match &quorum_data.quorum_type {
                QuorumType::Harvester => return Some(quorum_data.clone()),
                _ => {}
            }
        }
        return None
    }

    pub fn get_harvester_threshold(&self) -> usize {
        if let Some(data) = self.get_harvester_data() {
            return (data.members.len() as f64 * VALIDATION_THRESHOLD).ceil() as usize 
        }

        0usize
    }

    pub fn set_quorum_members(&mut self, quorums: Vec<(QuorumType, Vec<(NodeId, PublicKey)>)>) {
        self.0.clear();
        quorums.iter().for_each(|quorum| {
            let quorum_id = QuorumId::new(quorum.0.clone(), quorum.1.clone());
            let quorum_data = QuorumData {
                id: quorum_id.clone(),
                quorum_type: quorum.0.clone(),
                members: quorum.1.clone().into_iter().collect()
            };
            self.0.insert(quorum_id, quorum_data);
        });
    }
}

#[derive(Debug, Clone)]
pub struct SignerEngine {
    local_node_public_key: PublicKey,
    local_node_secret_key: SecretKey,
    quorum_members: QuorumMembers,
}

#[derive(thiserror::Error, Debug)]
pub struct Error;
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("signer error: {self:?}")
    }
}

impl SignerEngine {
    pub fn new(pub_key: PublicKey, sec_key: SecretKey) -> Self {
        Self {
            local_node_public_key: pub_key,
            local_node_secret_key: sec_key,
            quorum_members: QuorumMembers(HashMap::new()),
        }
    }
    /// transaction sign method
    pub fn sign<T: AsRef<[u8]>>(&mut self, data: T) -> Result<Signature, Error> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(data.as_ref());
        let result = hasher.finalize().to_vec();
        let message = Message::from_slice(&result);
        Ok(self
            .local_node_secret_key
            .sign_ecdsa(message.map_err(|_| Error)?))
    }

    /// signature verification
    pub fn verify<T: AsRef<[u8]>>(
        &self,
        node_id: &NodeId,
        sig: &Signature,
        data: &T,
    ) -> Result<(), Error> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(data.as_ref());
        let result = hasher.finalize().to_vec();
        let message = Message::from_slice(&result);
        let pk = self.quorum_members.get_public_key_from_members(node_id);
        if let Some(pk) = pk {
            return sig
                .verify(&message.map_err(|_| Error)?, &pk)
                .map_err(|_| Error);
        }

        Err(Error)
    }

    pub fn verify_batch<T: AsRef<[u8]>>(
        &self,
        batch_sigs: &[(NodeId, Signature)],
        data: &T,
    ) -> Result<(), Error> {
        if batch_sigs
            .iter()
            .map(|(node_id, sig)| self.verify(node_id, sig, data))
            .any(|res| res.is_err())
        {
            return Err(Error);
        }
        Ok(())
    }

    pub fn quorum_members(&self) -> QuorumMembers {
        self.quorum_members.clone()
    }

    pub fn public_key(&self) -> PublicKey {
        self.local_node_public_key.clone()
    }
}
