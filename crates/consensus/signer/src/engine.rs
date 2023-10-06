// secpt
use block::QuorumData;
use primitives::{NodeId, PublicKey, QuorumId, SecretKey, Signature};
use secp256k1::Message;
use sha2::Digest;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct QuorumMembers(pub HashMap<QuorumId, QuorumData>);
impl QuorumMembers {
    pub fn get_public_key_from_members(&self, k: &NodeId) -> Option<PublicKey> {
        for (_, quorum_data) in self.0.iter() {
            if let Some(pub_key) = quorum_data.members.get(k) {
                return Some(pub_key.clone());
            }
        }
        None
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
}
