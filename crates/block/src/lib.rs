pub mod block;
pub mod convergence_block;
pub mod genesis;
pub mod header;
pub mod invalid;
pub mod proposal_block;
pub mod vesting;

mod types;

pub use crate::{
    block::*, convergence_block::*, genesis::*, proposal_block::*, types::*, vesting::*,
};

pub mod valid {
    use primitives::{ByteVec, RawSignature, SignatureType};
    use serde::{Deserialize, Serialize};
    use utils::hash_data;
    use vrrb_core::transactions::Transaction;

    use crate::{ConvergenceBlock, GenesisBlock, ProposalBlock};

    pub trait Valid {
        type ValidationData;
        type DecodeError: std::error::Error;

        fn get_validation_data(&self) -> Result<Self::ValidationData, Self::DecodeError>;
        fn get_signature_type(&self) -> SignatureType;
        fn get_payload_hash(&self) -> ByteVec {
            vec![]
        }
        fn get_raw_signature(&self) -> Result<RawSignature, Self::DecodeError> {
            Ok(vec![])
        }
        fn get_node_idx(&self) -> Option<u16> {
            None
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub struct BlockValidationData {
        pub node_idx: Option<u16>,
        pub payload_hash: ByteVec,
        pub signature: RawSignature,
        pub signature_type: SignatureType,
    }

    impl<'a, T: Transaction<'a>> Valid for GenesisBlock<T> {
        type DecodeError = hex::FromHexError;
        type ValidationData = BlockValidationData;

        fn get_validation_data(&self) -> Result<Self::ValidationData, Self::DecodeError> {
            let signature = self.get_raw_signature()?;
            Ok(BlockValidationData {
                node_idx: self.get_node_idx(),
                payload_hash: self.get_payload_hash(),
                signature,
                signature_type: self.get_signature_type(),
            })
        }

        fn get_signature_type(&self) -> SignatureType {
            SignatureType::ThresholdSignature
        }

        fn get_payload_hash(&self) -> ByteVec {
            vec![]
        }

        fn get_raw_signature(&self) -> Result<RawSignature, Self::DecodeError> {
            if let Some(cert) = self.certificate.clone() {
                let signature = cert.decode_signature()?;
                Ok(signature)
            } else {
                Err(hex::FromHexError::InvalidStringLength)
            }
        }

        fn get_node_idx(&self) -> Option<u16> {
            None
        }
    }

    impl<'a, T: Transaction<'a>> Valid for ProposalBlock<T> {
        type DecodeError = hex::FromHexError;
        type ValidationData = BlockValidationData;

        fn get_validation_data(&self) -> Result<Self::ValidationData, Self::DecodeError> {
            let signature = self.get_raw_signature()?;
            Ok(BlockValidationData {
                node_idx: self.get_node_idx(),
                payload_hash: self.get_payload_hash(),
                signature,
                signature_type: self.get_signature_type(),
            })
        }

        fn get_signature_type(&self) -> SignatureType {
            SignatureType::PartialSignature
        }

        fn get_payload_hash(&self) -> ByteVec {
            let hashable_txns = self.get_hashable_txns();
            hash_data!(
                self.round,
                self.epoch,
                hashable_txns,
                self.claims,
                self.from
            )
            .to_vec()
        }

        fn get_raw_signature(&self) -> Result<RawSignature, Self::DecodeError> {
            let signature = self.decode_signature_share()?;
            Ok(signature.to_vec())
        }
    }

    impl Valid for ConvergenceBlock {
        type DecodeError = hex::FromHexError;
        type ValidationData = BlockValidationData;

        fn get_validation_data(&self) -> Result<Self::ValidationData, Self::DecodeError> {
            let signature = self.get_raw_signature()?;
            Ok(BlockValidationData {
                node_idx: self.get_node_idx(),
                payload_hash: self.get_payload_hash(),
                signature,
                signature_type: self.get_signature_type(),
            })
        }

        fn get_signature_type(&self) -> SignatureType {
            SignatureType::ThresholdSignature
        }

        fn get_raw_signature(&self) -> Result<RawSignature, Self::DecodeError> {
            if let Some(cert) = self.certificate.clone() {
                let signature = cert.decode_signature()?;
                Ok(signature)
            } else {
                Err(hex::FromHexError::InvalidStringLength)
            }
        }
    }
}
