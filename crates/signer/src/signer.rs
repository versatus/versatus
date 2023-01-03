//! This crate provides functionality for generating/verification of  partial
//! and threshold signatures
use std::{
    collections::BTreeMap,
    sync::{Arc, PoisonError, RwLock, RwLockReadGuard},
};

use dkg_engine::types::{config::ThresholdConfig, DkgState};
use hbbft::crypto::{Signature, SignatureShare, SIG_SIZE};
use primitives::NodeIdx;
use primitives::{PayloadHash as Hash, RawSignature, SignatureType};

use crate::types::{SignerError, SignerResult};

pub trait Signer {
    /// A function signature that takes a payload hash and returns a partial
    /// signature for the block from one of the node from the Quorum
    fn generate_partial_signature(&self, payload_hash: Hash) -> SignerResult<RawSignature>;

    /// This is the function that is used to generate the final signature for
    /// the block(t+1 non faulty nodes).
    fn generate_quorum_signature(
        &self,
        signature_shares: BTreeMap<NodeIdx, RawSignature>,
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

#[derive(Clone)]
pub struct SignatureProvider {
    pub dkg_state: Arc<RwLock<DkgState>>,
    pub quorum_config: ThresholdConfig,
}

impl From<PoisonError<RwLockReadGuard<'_, DkgState>>> for SignerError {
    fn from(_: PoisonError<RwLockReadGuard<'_, DkgState>>) -> SignerError {
        SignerError::DkgStateCannotBeRead
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
        let dkg_state = self.dkg_state.read()?;
        let secret_key_share = dkg_state.secret_key_share.as_ref();
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
    ///        let quorum_signature_result = sig_provider_node.generate_quorum_signature(sig_shares.clone());
    ///        assert_eq!(quorum_signature_result.is_err(), false);
    ///        assert_eq!(quorum_signature_result.unwrap().len() > 0, true);
    ///        break;
    ///    }

    ///   i=i-1;
    ///   }
    /// ```
    fn generate_quorum_signature(
        &self,
        signature_shares: BTreeMap<NodeIdx, RawSignature>,
    ) -> SignerResult<RawSignature> {
        if (signature_shares.len() as u16) < self.quorum_config.threshold {
            return Err(SignerError::ThresholdSignatureError(
                "Received less than t+1 signature shares".to_string(),
            ));
        }
        let dkg_state = self.dkg_state.read()?;
        for (_, sig) in signature_shares.iter() {
            if sig.len() != SIG_SIZE {
                return Err(SignerError::CorruptSignatureShare(
                    "Invalid Signature".to_string(),
                ));
            }
        }
        // The below code is converting the signature shares from a map of bytes to a
        // map of signature shares.
        let sig_shares: BTreeMap<usize, SignatureShare> = signature_shares
            .iter()
            .map(|(x, sig_share_bytes)| {
                let signature_arr: [u8; 96] = sig_share_bytes.clone().try_into().unwrap();
                let sig_share_result = SignatureShare::from_bytes(signature_arr).unwrap();
                (*x as usize, sig_share_result)
            })
            .collect();

        let result = dkg_state.public_key_set.as_ref();
        //Construction of combining t+1 valid shares to form threshold
        let combine_signature_result = match result {
            Some(pub_key_set) => pub_key_set.combine_signatures(&sig_shares),
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
        let dkg_state = self.dkg_state.read()?;
        match signature_type {
            SignatureType::PartialSignature => {
                let public_key_share_opt = dkg_state.public_key_set.clone();
                let public_key_share = match public_key_share_opt {
                    Some(public_key_share) => public_key_share.public_key_share(node_idx as usize),
                    None => return Err(SignerError::GroupPublicKeyMissing),
                };
                let signature_arr: [u8; 96] = signature.try_into().unwrap();
                let sig_share_result = SignatureShare::from_bytes(signature_arr);
                let sig_share = match sig_share_result {
                    Ok(sig_share) => sig_share,
                    Err(e) => {
                        return Err(SignerError::SignatureVerificationError(format!(
                            "Error parsing partial signature details : {:?}",
                            e
                        )))
                    },
                };
                Ok(public_key_share.verify(&sig_share, payload_hash))
            },
            SignatureType::ThresholdSignature | SignatureType::ChainLockSignature => {
                let public_key_set_opt = dkg_state.public_key_set.clone();
                if public_key_set_opt.is_none() {
                    return Err(SignerError::GroupPublicKeyMissing);
                }
                let public_key_set = match public_key_set_opt {
                    Some(public_key_set) => public_key_set,
                    None => return Err(SignerError::GroupPublicKeyMissing),
                };
                let signature_arr: [u8; 96] = signature.try_into().unwrap();
                let sig_share_result = Signature::from_bytes(signature_arr);
                let signature = match sig_share_result {
                    Ok(signature) => signature,
                    Err(e) => {
                        return Err(SignerError::SignatureVerificationError(format!(
                            "Error parsing threshold signature details : {:?}",
                            e
                        )))
                    },
                };

                Ok(public_key_set.public_key().verify(&signature, payload_hash))
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use std::collections::BTreeMap;

    use dkg_engine::{test_utils::generate_dkg_engine_with_states, types::config::ThresholdConfig};
    use primitives::{is_enum_variant, types::SignatureType};

    use crate::{
        signer::{SignatureProvider, Signer},
        types::SignerError,
    };

    #[tokio::test]
    #[ignore = "temporarily broken because of changes in both node and dkg"]
    async fn successful_test_generation_partial_signature() {
        let dkg_engine_node = generate_dkg_engine_with_states().await.pop().unwrap();
        let message = "This is test message";
        let sig_provider = SignatureProvider {
            dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine_node.dkg_state)),
            quorum_config: ThresholdConfig {
                threshold: 1,
                upper_bound: 4,
            },
        };
        let result = sig_provider.generate_partial_signature(message.as_bytes().to_vec());
        match result {
            Ok(sig_share) => assert_eq!(sig_share.len() > 0, true),
            Err(e) => panic!("{}", format!("Generating partial signature failed {:?}", e)),
        }
    }

    #[tokio::test]
    #[ignore = "temporarily broken because of changes in both node and dkg"]
    async fn failed_test_generation_partial_signature() {
        let mut dkg_engines = generate_dkg_engine_with_states().await;
        let mut dkg_engine_node = dkg_engines.pop().unwrap();
        let message = "This is test message";
        dkg_engine_node.dkg_state.secret_key_share = None;
        let sig_provider = SignatureProvider {
            dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine_node.dkg_state)),
            quorum_config: ThresholdConfig {
                threshold: 1,
                upper_bound: 4,
            },
        };
        let result = sig_provider.generate_partial_signature(message.as_bytes().to_vec());
        assert_eq!(
            result,
            Err(crate::types::SignerError::SecretKeyShareMissing)
        );
    }

    #[tokio::test]
    #[ignore = "temporarily broken because of changes in both node and dkg"]
    async fn successful_test_generation_quorum_signature() {
        let mut dkg_engines = generate_dkg_engine_with_states().await;
        let mut sig_shares = BTreeMap::new();
        let message = "This is test message";
        let mut i: u16 = 3;
        while !dkg_engines.is_empty() {
            let dkg_engine_node = dkg_engines.pop().unwrap();

            let sig_provider_node = SignatureProvider {
                dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine_node.dkg_state)),
                quorum_config: ThresholdConfig {
                    threshold: 1,
                    upper_bound: 4,
                },
            };
            let signature_share_node = sig_provider_node
                .generate_partial_signature(message.as_bytes().to_vec())
                .unwrap();
            sig_shares.insert(i, signature_share_node);
            if i == 0 {
                let quorum_signature_result =
                    sig_provider_node.generate_quorum_signature(sig_shares.clone());
                assert_eq!(quorum_signature_result.is_err(), false);
                assert_eq!(quorum_signature_result.unwrap().len() > 0, true);
                break;
            }

            i = i - 1;
        }
    }

    #[tokio::test]
    #[ignore = "temporarily broken because of changes in both node and dkg"]
    async fn successful_verification_partial_signature() {
        let dkg_engine_node = generate_dkg_engine_with_states().await.pop().unwrap();
        let message = "This is test message";
        let sig_provider = SignatureProvider {
            dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine_node.dkg_state)),
            quorum_config: ThresholdConfig {
                threshold: 1,
                upper_bound: 4,
            },
        };

        let signature_share = sig_provider
            .generate_partial_signature(message.as_bytes().to_vec())
            .unwrap();

        let sig_status = sig_provider.verify_signature(
            3,
            message.as_bytes().to_vec(),
            signature_share,
            SignatureType::PartialSignature,
        );
        assert_eq!(sig_status.is_err(), false);
        if !sig_status.is_err() {
            assert_eq!(sig_status.unwrap(), true);
        }
    }

    #[tokio::test]
    #[ignore = "temporarily broken because of changes in both node and dkg"]
    async fn successful_verification_threshold_signature() {
        let message = "This is test message";
        let mut dkg_engines = generate_dkg_engine_with_states().await;
        let mut sig_shares = BTreeMap::new();
        let mut i: u16 = 3;
        while !dkg_engines.is_empty() {
            let dkg_engine_node = dkg_engines.pop().unwrap();

            let sig_provider_node = SignatureProvider {
                dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine_node.dkg_state)),
                quorum_config: ThresholdConfig {
                    threshold: 1,
                    upper_bound: 4,
                },
            };
            let signature_share_node = sig_provider_node
                .generate_partial_signature(message.as_bytes().to_vec())
                .unwrap();
            sig_shares.insert(i, signature_share_node);
            if i == 0 {
                let threshold_sig_result = sig_provider_node.generate_quorum_signature(sig_shares);
                let sig = threshold_sig_result.unwrap();
                let sig_status = sig_provider_node.verify_signature(
                    2,
                    message.as_bytes().to_vec(),
                    sig,
                    SignatureType::ThresholdSignature,
                );

                assert_eq!(sig_status.is_err(), false);
                if !sig_status.is_err() {
                    assert_eq!(sig_status.unwrap(), true);
                }
                break;
            }
            i = i - 1;
        }
    }

    #[tokio::test]
    #[ignore = "temporarily broken because of changes in both node and dkg"]
    async fn failed_verification_threshold_signature() {
        let message = "This is test message";

        let mut dkg_engines = generate_dkg_engine_with_states().await;
        let dkg_engine_node = dkg_engines.pop().unwrap();

        let sig_provider = SignatureProvider {
            dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine_node.dkg_state)),
            quorum_config: ThresholdConfig {
                threshold: 1,
                upper_bound: 4,
            },
        };

        let sig_status = sig_provider.verify_signature(
            2,
            message.as_bytes().to_vec(),
            [0u8; 96].to_vec(),
            SignatureType::ThresholdSignature,
        );
        assert_eq!(sig_status.is_err(), true);
        assert!(is_enum_variant!(
            sig_status,
            Err(SignerError::SignatureVerificationError { .. })
        ));
    }
}
