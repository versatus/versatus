pub mod block;
pub mod genesis;
pub mod header;
pub mod invalid;
pub use crate::block::*;

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::UNIX_EPOCH};

    use rand::Rng;
    use reward::reward::Reward;
    use ritelinked::LinkedHashMap;
    use vrrb_core::{
        claim::Claim,
        keypair::KeyPair,
        txn::{NewTxnArgs, Txn},
    };

    use crate::{
        header::BlockHeader,
        Block, 
        MineArgs,
        ConvergenceBlock,
        ProposalBlock,
        GenesisBlock,
        Conflict,
    };

    #[ignore]
    #[test]
    fn test_create_genesis_block() {
        todo!()
    }

    #[ignore]
    #[test]
    fn test_create_proposal_block() {
        todo!()
    }

    #[ignore]
    #[test]
    fn test_create_convergence_block_no_conflicts() {
        todo!()
    }

    #[ignore]
    #[test]
    fn test_create_convergence_block_conflicts() {
        todo!()
    }

    #[ignore]
    #[test]
    fn test_resolve_conflicts_valid() {
        todo!()
    }

    #[ignore]
    #[test]
    fn test_epoch_change() {
        todo!()
    }

    #[ignore]
    #[test]
    fn test_utility_adjustment() {
        todo!()
    }
}
