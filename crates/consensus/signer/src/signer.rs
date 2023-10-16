//! This crate provides functionality for generating/verification of  partial
//! and threshold signatures
use std::{collections::BTreeMap, str::FromStr};

// use dkg_engine::prelude::*;
use hbbft::{
    crypto::{Fr, FrRepr, PublicKeySet, SecretKeyShare, Signature, SignatureShare, SIG_SIZE},
    pairing::PrimeField,
};
use primitives::{NodeId, NodeIdx, PayloadHash as Hash, RawSignature, SignatureType};
use vrrb_config::ThresholdConfig;

use crate::types::{SignerError, SignerResult};

pub trait Signer {
    /// A function signature that takes a payload hash and returns a partial
    /// signature for the block from one of the node from the Quorum
    fn generate_partial_signature(&self, payload_hash: Hash) -> SignerResult<RawSignature>;

    /// This is the function that is used to generate the final signature for
    /// the block(t+1 non faulty nodes).
    fn generate_quorum_signature(
        &self,
        quorum_threshold: u16,
        signature_shares: BTreeMap<NodeId, RawSignature>,
    ) -> SignerResult<RawSignature>;

    /// This function is used to verify the signature of the block.
    fn verify_signature(
        &self,
        node_idx: NodeIdx,
        payload_hash: Hash,
        signature: RawSignature,
        signature_type: SignatureType,
    ) -> SignerResult<bool>;
}

// TODO: make this cleaner if possible
/// Converts a string slice into the Fr type.
pub trait NodeIdFrBuilder: AsRef<str> {
    fn create_fr(&self) -> Fr {
        let (m, l) = uuid::Uuid::from_str(&self.as_ref())
            .expect("failed to create uuid from string slice")
            .as_u64_pair();
        let uuid_vec = vec![m, l, 0, 0];
        let mut fr_repr = FrRepr::default();
        for (mut fr_slot, uuid_slot) in fr_repr.0.iter().zip(uuid_vec.iter()) {
            fr_slot = uuid_slot;
        }
        Fr::from_repr(fr_repr).expect("failed to create Fr from FrRepr")
    }
}
impl NodeIdFrBuilder for NodeId {}

#[derive(Clone, Debug)]
pub struct SignatureProvider {
    public_key_set: Option<PublicKeySet>,
    secret_key_share: Option<SecretKeyShare>,
    pub quorum_config: ThresholdConfig,
}

// impl From<&DkgState> for SignatureProvider {
//     fn from(item: &DkgState) -> SignatureProvider {
//         let public_key_set = item.public_key_set().clone();
//         let secret_key_share = item.secret_key_share().clone();

//         let mut sig_provider = SignatureProvider::new(ThresholdConfig::default());

//         match public_key_set {
//             Some(pks) => sig_provider.set_public_key_set(pks),
//             _ => {},
//         }
//         match secret_key_share {
//             Some(sks) => sig_provider.set_secret_key_share(sks),
//             _ => {},
//         }

//         sig_provider
//     }
// }

// impl From<PoisonError<RwLockReadGuard<'_, DkgState>>> for SignerError {
//     fn from(_: PoisonError<RwLockReadGuard<'_, DkgState>>) -> SignerError {
//         SignerError::DkgStateCannotBeRead
//     }
// }

impl SignatureProvider {
    pub fn new(quorum_config: ThresholdConfig) -> Self {
        Self {
            public_key_set: None,
            secret_key_share: None,
            quorum_config,
        }
    }

    pub fn set_public_key_set(&mut self, public_key_set: PublicKeySet) {
        self.public_key_set = Some(public_key_set.clone());
    }

    pub fn set_secret_key_share(&mut self, secret_key_share: SecretKeyShare) {
        self.secret_key_share = Some(secret_key_share.clone());
    }

    pub fn set_threshold_config(&mut self, threshold_config: ThresholdConfig) {
        self.quorum_config = threshold_config;
    }

    pub fn secret_key_share(&self) -> Option<SecretKeyShare> {
        self.secret_key_share.clone()
    }

    pub fn public_key_set(&self) -> Option<PublicKeySet> {
        self.public_key_set.clone()
    }

    pub fn quorum_config(&self) -> ThresholdConfig {
        self.quorum_config.clone()
    }
}

impl Signer for SignatureProvider {
    /// > This function takes a `payload hash` and returns a `partial signature`
    ///
    /// Arguments:
    ///
    /// * `payload_hash`: The hash of the payload to be signed.
    ///
    /// Returns:
    ///
    /// * A signature of the payload hash.
    ///
    /// # Examples
    /// ```
    /// use dkg_engine::{test_utils::generate_dkg_engine_with_states, types::config::ThresholdConfig};
    /// use signer::signer::{SignatureProvider, Signer};
    ///
    /// // Construct the instance to DkgEngine
    /// let dkg_engine_node = generate_dkg_engine_with_states().pop().unwrap();
    /// let message = "This is test message";
    ///
    /// // Signature Provider struct that necessary to generate partial/threshold signatures
    /// let sig_provider = SignatureProvider {
    ///     dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine_node.dkg_state)),
    ///     quorum_config: ThresholdConfig {
    ///         threshold: 1,
    ///         upper_bound: 4,
    ///     },
    /// };
    /// let result = sig_provider.generate_partial_signature(message.as_bytes().to_vec());
    /// match result {
    ///     Ok(sig_share) => assert_eq!(sig_share.len() > 0, true),
    ///     Err(e) => panic!("{}", format!("Generating partial signature failed {:?}", e)),
    /// }
    /// ```
    fn generate_partial_signature(&self, payload_hash: Hash) -> SignerResult<RawSignature> {
        let secret_key_share = self.secret_key_share();
        let secret_key_share = match secret_key_share {
            Some(key) => key,
            None => return Err(SignerError::SecretKeyShareMissing),
        };

        let signature = secret_key_share.sign(payload_hash);
        let sig = signature.to_bytes().to_vec();

        Ok(sig)
    }

    /// > This function takes in a `map of node-ids to signature shares`, and
    /// > returns a `quorum signature`
    /// if the number of signature shares is greater than or equal to the
    /// threshold
    ///
    /// Arguments:
    ///
    /// * `signature_shares`: A map of node ids to signature shares.
    ///
    /// Returns:
    ///
    /// * A threshold quorum signature
    ///
    /// # Examples
    /// ```
    ///  use dkg_engine::{test_utils::generate_dkg_engine_with_states, types::config::ThresholdConfig};
    ///  use signer::signer::SignatureProvider;
    ///  use hbbft::crypto::SignatureShare;
    ///  use crate::signer::signer::Signer;
    ///   
    ///   let mut dkg_engines = generate_dkg_engine_with_states();
    ///   let mut sig_shares = std::collections::BTreeMap::new();
    ///   let message = "This is test message";
    ///   let mut i:u16=3;
    ///
    ///   while !dkg_engines.is_empty(){
    ///    let dkg_engine_node = dkg_engines.pop().unwrap();
    ///
    ///    // Signature Provider struct that necessary to generate partial/threshold signatures
    ///    let sig_provider_node = SignatureProvider {
    ///        dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine_node.dkg_state)),
    ///        quorum_config: ThresholdConfig {
    ///            threshold: 1,
    ///            upper_bound: 4,
    ///        },
    ///    };
    ///
    ///    //Partial signature
    ///    let signature_share_node = sig_provider_node
    ///    .generate_partial_signature(message.as_bytes().to_vec()).unwrap();
    ///     sig_shares.insert(i, signature_share_node);
    ///    if i==0{
    ///        //Populate signature shares from all t+1 nodes in the quorum
    ///        let quorum_signature_result = sig_provider_node.generate_quorum_signature(1,sig_shares.clone());
    ///        assert_eq!(quorum_signature_result.is_err(), false);
    ///        assert_eq!(quorum_signature_result.unwrap().len() > 0, true);
    ///        break;
    ///    }

    ///   i=i-1;
    ///   }
    /// ```
    fn generate_quorum_signature(
        &self,
        quorum_threshold: u16,
        signature_shares: BTreeMap<NodeId, RawSignature>,
    ) -> SignerResult<RawSignature> {
        if (signature_shares.len() as u16) < quorum_threshold {
            return Err(SignerError::ThresholdSignatureError(
                "Received less than t+1 signature shares".to_string(),
            ));
        }
        for (_, sig) in signature_shares.iter() {
            if sig.len() != SIG_SIZE {
                return Err(SignerError::CorruptSignatureShare(
                    "Invalid Signature".to_string(),
                ));
            }
        }
        // The below code is converting the signature shares from a map of bytes to a
        // map of signature shares.
        let mut sig_shares: Vec<(Fr, SignatureShare)> = Vec::new();
        for (node_id, sig_share_bytes) in signature_shares.iter() {
            match TryInto::<[u8; 96]>::try_into(sig_share_bytes.as_slice()) {
                Ok(signature_arr) => {
                    if let Ok(sig_share_result) = SignatureShare::from_bytes(signature_arr) {
                        sig_shares.push((node_id.create_fr(), sig_share_result));
                    }
                },
                Err(_) => continue,
            }
        }

        let result = self.public_key_set();
        //Construction of combining t+1 valid shares to form threshold
        let combine_signature_result = match result {
            // TODO figure out how to turn strings into IntoFr
            Some(pub_key_set) => {
                let shares = sig_shares.iter().map(|(idx, sig)| (idx, sig));
                pub_key_set.combine_signatures(shares)
            },
            None => return Err(SignerError::GroupPublicKeyMissing),
        };

        let sig = match combine_signature_result {
            Ok(sig) => sig,
            Err(e) => {
                return Err(SignerError::ThresholdSignatureError(format!(
                    "Error while constructing threshold signature details: {:?}",
                    e.to_string()
                )))
            },
        };
        Ok(sig.to_bytes().to_vec())
    }

    /// > This function is used to verify either partial or quorum signature for
    /// > the block
    ///
    /// Arguments:
    ///
    /// * `node_idx`: Node Index in the chain
    /// * `payload_hash`: payload hash of the block
    /// * `signature`: signature to be verified
    /// * `signature_type`: Type of Signature (Partial/Threshold/ChainLock)
    ///
    /// Returns:
    ///
    /// * Verification Status (bool)
    /// # Examples
    /// ```
    /// // Construct the instance to DkgEngine
    /// use dkg_engine::{
    ///     test_utils::generate_dkg_engine_with_states,
    ///     types::{config::ThresholdConfig, DkgEngine},
    /// };
    /// use hbbft::crypto::SignatureShare;
    /// use primitives::SignatureType;
    /// use signer::signer::SignatureProvider;
    ///
    /// use crate::signer::signer::Signer;
    ///
    /// let mut dkg_engines = generate_dkg_engine_with_states();
    /// let mut sig_shares = std::collections::BTreeMap::new();
    /// let message = "This is test message";
    /// let mut i: u16 = 3;
    /// while !dkg_engines.is_empty() {
    ///     let dkg_engine_node = dkg_engines.pop().unwrap();
    ///
    ///     // Signature Provider struct that necessary to generate partial/threshold signatures
    ///     let sig_provider_node = SignatureProvider {
    ///         dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine_node.dkg_state)),
    ///         quorum_config: ThresholdConfig {
    ///             threshold: 1,
    ///             upper_bound: 4,
    ///         },
    ///     };
    ///     //Generate Partial Signature
    ///     let signature_share_node = sig_provider_node
    ///         .generate_partial_signature(message.as_bytes().to_vec())
    ///         .unwrap();
    ///     sig_shares.insert(i as u16, signature_share_node);
    ///     if i == 0 {
    ///         //Populate signature shares from all t+1 nodes in the quorum
    ///         let threshold_sig_result = sig_provider_node.generate_quorum_signature(sig_shares);
    ///         let sig = threshold_sig_result.unwrap();
    ///         let sig_status = sig_provider_node.verify_signature(
    ///             2,
    ///             message.as_bytes().to_vec(),
    ///             sig,
    ///             SignatureType::ThresholdSignature,
    ///         );
    ///         assert_eq!(sig_status.is_err(), false);
    ///         if !sig_status.is_err() {
    ///             assert_eq!(sig_status.unwrap(), true);
    ///         }
    ///         break;
    ///     }
    ///     i = i - 1;
    /// }
    /// ```
    fn verify_signature(
        &self,
        node_idx: NodeIdx,
        payload_hash: Hash,
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
                let public_key_share_opt = self.public_key_set();
                let public_key_share = match public_key_share_opt {
                    Some(public_key_share) => public_key_share.public_key_share(node_idx as usize),
                    None => return Err(SignerError::GroupPublicKeyMissing),
                };
                if let Ok(signature_arr) = TryInto::<[u8; 96]>::try_into(signature.as_slice()) {
                    if let Ok(sig_share) = SignatureShare::from_bytes(signature_arr) {
                        Ok(public_key_share.verify(&sig_share, payload_hash))
                    } else {
                        Err(SignerError::CorruptSignatureShare(String::from(
                            "Corrupt signature share",
                        )))
                    }
                } else {
                    Err(SignerError::PartialSignatureError(String::from(
                        "Signature must be 96 byte array",
                    )))
                }
            },
            SignatureType::ThresholdSignature | SignatureType::ChainLockSignature => {
                let public_key_set_opt = self.public_key_set();
                if public_key_set_opt.is_none() {
                    return Err(SignerError::GroupPublicKeyMissing);
                }
                let public_key_set = match public_key_set_opt {
                    Some(public_key_set) => public_key_set,
                    None => return Err(SignerError::GroupPublicKeyMissing),
                };
                if let Ok(signature_arr) = TryInto::<[u8; 96]>::try_into(signature.as_slice()) {
                    if let Ok(signature) = Signature::from_bytes(signature_arr) {
                        Ok(public_key_set.public_key().verify(&signature, payload_hash))
                    } else {
                        Err(SignerError::ThresholdSignatureError(String::from(
                            "Corrupt signature",
                        )))
                    }
                } else {
                    Err(SignerError::ThresholdSignatureError(String::from(
                        "Signature must be 96 byte array",
                    )))
                }
            },
        }
    }
}

// #[cfg(test)]
// mod tests {

//     use dkg_engine::test_utils::generate_dkg_engine_with_states;
//     use primitives::SignatureType;
//     use std::collections::BTreeMap;
//     use vrrb_config::ThresholdConfig;
//     use vrrb_core::is_enum_variant;

//     use crate::{
//         signer::{SignatureProvider, Signer},
//         types::SignerError,
//     };

//     #[tokio::test]
//     #[ignore = "https://github.com/versatus/versatus/issues/477"]
//     async fn successful_test_generation_partial_signature() {
//         let dkg_engine_node = generate_dkg_engine_with_states().await.pop().unwrap();
//         let message = "This is test message";
//         let threshold_config = ThresholdConfig {
//             upper_bound: 4,
//             threshold: 1,
//         };
//         let mut sig_provider = SignatureProvider::from(&dkg_engine_node.dkg_state);
//         sig_provider.set_threshold_config(threshold_config);
//         let result = sig_provider.generate_partial_signature(message.as_bytes().to_vec());
//         match result {
//             Ok(sig_share) => assert_eq!(sig_share.len() > 0, true),
//             Err(e) => panic!("{}", format!("Generating partial signature failed {:?}", e)),
//         }
//     }

//     #[tokio::test]
//     async fn failed_test_generation_partial_signature() {
//         let mut dkg_engines = generate_dkg_engine_with_states().await;
//         let mut dkg_engine_node = dkg_engines.pop().unwrap();
//         let message = "This is test message";
//         let threshold_config = ThresholdConfig {
//             threshold: 1,
//             upper_bound: 4,
//         };
//         dkg_engine_node.dkg_state.set_secret_key_share(None);
//         let mut sig_provider = SignatureProvider::from(&dkg_engine_node.dkg_state);
//         sig_provider.set_threshold_config(threshold_config);
//         let result = sig_provider.generate_partial_signature(message.as_bytes().to_vec());
//         assert_eq!(result, Err(SignerError::SecretKeyShareMissing));
//     }

//     #[tokio::test]
//     #[ignore = "https://github.com/versatus/versatus/issues/480"]
//     async fn successful_test_generation_quorum_signature() {
//         let mut dkg_engines = generate_dkg_engine_with_states().await;
//         let mut sig_shares = BTreeMap::new();
//         let message = "This is test message";
//         let mut i: u16 = 3;
//         while !dkg_engines.is_empty() {
//             let dkg_engine_node = dkg_engines.pop().unwrap();

//             let mut sig_provider_node = SignatureProvider::from(&dkg_engine_node.dkg_state);
//             let threshold_config = ThresholdConfig {
//                 threshold: 1,
//                 upper_bound: 4,
//             };
//             sig_provider_node.set_threshold_config(threshold_config);

//             let signature_share_node = sig_provider_node
//                 .generate_partial_signature(message.as_bytes().to_vec())
//                 .unwrap();
//             sig_shares.insert(format!("node-{i}"), signature_share_node);
//             if i == 0 {
//                 let quorum_signature_result =
//                     sig_provider_node.generate_quorum_signature(1, sig_shares.clone());
//                 assert_eq!(quorum_signature_result.is_err(), false);
//                 assert_eq!(quorum_signature_result.unwrap().len() > 0, true);
//                 break;
//             }

//             i = i - 1;
//         }
//     }

//     #[tokio::test]
//     #[ignore = "https://github.com/versatus/versatus/issues/478"]
//     async fn successful_verification_partial_signature() {
//         let dkg_engine_node = generate_dkg_engine_with_states().await.pop().unwrap();
//         let message = "This is test message";
//         let mut sig_provider = SignatureProvider::from(&dkg_engine_node.dkg_state);
//         sig_provider.set_threshold_config(ThresholdConfig {
//             threshold: 1,
//             upper_bound: 4,
//         });

//         let signature_share = sig_provider
//             .generate_partial_signature(message.as_bytes().to_vec())
//             .unwrap();

//         let sig_status = sig_provider.verify_signature(
//             3,
//             message.as_bytes().to_vec(),
//             signature_share,
//             SignatureType::PartialSignature,
//         );
//         assert_eq!(sig_status.is_err(), false);
//         if !sig_status.is_err() {
//             assert_eq!(sig_status.unwrap(), true);
//         }
//     }

//     #[tokio::test]
//     #[ignore = "https://github.com/versatus/versatus/issues/476"]
//     async fn successful_verification_threshold_signature() {
//         let message = "This is test message";
//         let mut dkg_engines = generate_dkg_engine_with_states().await;
//         let mut sig_shares = BTreeMap::new();
//         let mut i: u16 = 3;
//         while !dkg_engines.is_empty() {
//             let dkg_engine_node = dkg_engines.pop().unwrap();
//             let mut sig_provider_node = SignatureProvider::from(&dkg_engine_node.dkg_state);
//             sig_provider_node.set_threshold_config(ThresholdConfig {
//                 threshold: 1,
//                 upper_bound: 4,
//             });

//             let signature_share_node = sig_provider_node
//                 .generate_partial_signature(message.as_bytes().to_vec())
//                 .unwrap();
//             sig_shares.insert(format!("node-{i}"), signature_share_node);
//             if i == 0 {
//                 let threshold_sig_result =
//                     sig_provider_node.generate_quorum_signature(1, sig_shares);
//                 let sig = threshold_sig_result.unwrap();
//                 let sig_status = sig_provider_node.verify_signature(
//                     2,
//                     message.as_bytes().to_vec(),
//                     sig,
//                     SignatureType::ThresholdSignature,
//                 );

//                 assert_eq!(sig_status.is_err(), false);
//                 if !sig_status.is_err() {
//                     assert_eq!(sig_status.unwrap(), true);
//                 }
//                 break;
//             }
//             i = i - 1;
//         }
//     }

//     #[tokio::test]
//     #[ignore = "https://github.com/versatus/versatus/issues/479"]
//     async fn failed_verification_threshold_signature() {
//         let message = "This is test message";

//         let mut dkg_engines = generate_dkg_engine_with_states().await;
//         let dkg_engine_node = dkg_engines.pop().unwrap();

//         let mut sig_provider = SignatureProvider::from(&dkg_engine_node.dkg_state);

//         sig_provider.set_threshold_config(ThresholdConfig {
//             threshold: 1,
//             upper_bound: 4,
//         });

//         let sig_status = sig_provider.verify_signature(
//             2,
//             message.as_bytes().to_vec(),
//             [0u8; 96].to_vec(),
//             SignatureType::ThresholdSignature,
//         );
//         assert_eq!(sig_status.is_err(), true);
//         assert!(is_enum_variant!(
//             sig_status,
//             Err(SignerError::ThresholdSignatureError { .. })
//         ));
//     }
// }
