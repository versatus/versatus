pub mod block;
pub mod convergence_block;
pub mod genesis;
pub mod header;
pub mod invalid;
pub mod proposal_block;
pub mod vesting;

mod types;

pub use crate::{
    block::*,
    convergence_block::*,
    genesis::*,
    proposal_block::*,
    types::*,
    vesting::*,
};

pub mod valid {
    use primitives::{ByteVec, RawSignature, SignatureType};
    use serde::{Serialize, Deserialize};

    use crate::{GenesisBlock, ProposalBlock, ConvergenceBlock};

    pub trait Valid {
        type ValidationData;
        fn get_validation_data(&self) -> Self::ValidationData; 
        fn get_signature_type(&self) -> SignatureType;
        fn get_payload_hash(&self) -> ByteVec {
            vec![]
        }
        fn get_raw_signature(&self) -> RawSignature {
            vec![]
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
        pub signature_type: SignatureType
    }

    impl Valid for GenesisBlock {
        type ValidationData = BlockValidationData;

        fn get_validation_data(&self) -> Self::ValidationData {
            BlockValidationData {
                node_idx: self.get_node_idx(),
                payload_hash: self.get_payload_hash(),
                signature: self.get_raw_signature(),
                signature_type: self.get_signature_type()
            }
        }

        fn get_signature_type(&self) -> SignatureType {
            SignatureType::ThresholdSignature
        }

    }

    impl Valid for ProposalBlock {
        type ValidationData = BlockValidationData;

        fn get_validation_data(&self) -> Self::ValidationData {
            BlockValidationData {
                node_idx: self.get_node_idx(),
                payload_hash: self.get_payload_hash(),
                signature: self.get_raw_signature(),
                signature_type: self.get_signature_type()
            }
        }

        fn get_signature_type(&self) -> SignatureType {
            SignatureType::PartialSignature
        }
    }

    impl Valid for ConvergenceBlock {
        type ValidationData = BlockValidationData;

        fn get_validation_data(&self) -> Self::ValidationData {
            BlockValidationData {
                node_idx: self.get_node_idx(),
                payload_hash: self.get_payload_hash(),
                signature: self.get_raw_signature(),
                signature_type: self.get_signature_type(),
            }
        }

        fn get_signature_type(&self) -> SignatureType {
            SignatureType::ThresholdSignature
        }
    }
}
