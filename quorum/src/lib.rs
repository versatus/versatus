pub  mod election;
pub mod quorum;
pub mod dummyNode;
pub mod dummyChildBlock;

use std::env;
fn main() {
  // this method needs to be inside main() method
  env::set_var("RUST_BACKTRACE", "1");
}

#[cfg(test)]
mod tests {
    use crate::dummyChildBlock::DummyChildBlock;
    use crate::dummyNode::DummyNode;
    use claim::claim::Claim;
    use crate::election::Election;
    use crate::quorum::Quorum;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn not_enough_nodes() {
        let mut dummyNodes: Vec<DummyNode> = Vec::new();
        let node1: DummyNode = DummyNode::new(b"node1");
        let node2: DummyNode = DummyNode::new(b"node2");
        let node3: DummyNode = DummyNode::new(b"node3");
        dummyNodes.push(node1.clone());
        dummyNodes.push(node2.clone());
        dummyNodes.push(node3.clone());
  
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();
        let claim1: Claim = Claim::new(node1.pubkey.clone(), addr.clone(), 1);
        let claim2: Claim = Claim::new(node2.pubkey.clone(), addr.clone(), 2);
        let claim3: Claim = Claim::new(node3.pubkey.clone(), addr.clone(), 3);
        dummyClaims.push(claim1);
        dummyClaims.push(claim2);
        dummyClaims.push(claim3);

        let child_block = DummyChildBlock::new(b"one", b"two");
        let mut quorum: Quorum = Quorum::new();

        assert!(quorum.run_election(&child_block, dummyClaims, dummyNodes).is_err());
    }

    #[test]
    fn invalid_block_height() {
        let mut dummyNodes: Vec<DummyNode> = Vec::new();
        let node1: DummyNode = DummyNode::new(b"node1");
        let node2: DummyNode = DummyNode::new(b"node2");
        let node3: DummyNode = DummyNode::new(b"node3");
        dummyNodes.push(node1.clone());
        dummyNodes.push(node2.clone());
        dummyNodes.push(node3.clone());
  
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();
        let claim1: Claim = Claim::new(node1.pubkey.clone(), addr.clone(), 1);
        let claim2: Claim = Claim::new(node2.pubkey.clone(), addr.clone(), 2);
        let claim3: Claim = Claim::new(node3.pubkey.clone(), addr.clone(), 3);
        dummyClaims.push(claim1);
        dummyClaims.push(claim2);
        dummyClaims.push(claim3);

        let mut child_block = DummyChildBlock::new(b"one", b"two");
        child_block.set_block_height(0);
        let mut quorum: Quorum = Quorum::new();
        assert!(quorum.run_election(&child_block, dummyClaims, dummyNodes).is_err());
        
    }

    #[test]
    fn invalid_block_timestamp() {
        let mut dummyNodes: Vec<DummyNode> = Vec::new();
        let node1: DummyNode = DummyNode::new(b"node1");
        let node2: DummyNode = DummyNode::new(b"node2");
        let node3: DummyNode = DummyNode::new(b"node3");
        dummyNodes.push(node1.clone());
        dummyNodes.push(node2.clone());
        dummyNodes.push(node3.clone());

  
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();
        let claim1: Claim = Claim::new(node1.pubkey.clone(), addr.clone(), 1);
        let claim2: Claim = Claim::new(node2.pubkey.clone(), addr.clone(), 2);
        let claim3: Claim = Claim::new(node3.pubkey.clone(), addr.clone(), 3);
        dummyClaims.push(claim1);
        dummyClaims.push(claim2);
        dummyClaims.push(claim3);

        let mut child_block = DummyChildBlock::new(b"one", b"two");
        child_block.set_block_timestamp(0);
        let mut quorum: Quorum = Quorum::new();
        assert!(quorum.run_election(&child_block, dummyClaims, dummyNodes).is_err());
        
    }

    #[test]
    fn elect_quorum() {
        let mut dummyNodes: Vec<DummyNode> = Vec::new();
        //just iterate to make new ones
        //to get node pubkey index into dummyNodes vector
        //pass msg w format string (enumerate)
        let node1: DummyNode = DummyNode::new(b"nodeOne");
        let node2: DummyNode = DummyNode::new(b"nodeTwo");
        let node3: DummyNode = DummyNode::new(b"nodeThree");
        let node4: DummyNode = DummyNode::new(b"nodeFour");
        let node5: DummyNode = DummyNode::new(b"nodeFive");
        let node6: DummyNode = DummyNode::new(b"nodeSix");
        let node7: DummyNode = DummyNode::new(b"nodeSeven");
        let node8: DummyNode = DummyNode::new(b"nodeEight");
        let node9: DummyNode = DummyNode::new(b"nodeNine");
        let node10: DummyNode = DummyNode::new(b"nodeTen");
        dummyNodes.push(node1.clone());
        dummyNodes.push(node2.clone());
        dummyNodes.push(node3.clone());
        dummyNodes.push(node4.clone());
        dummyNodes.push(node5.clone());
        dummyNodes.push(node6.clone());
        dummyNodes.push(node7.clone());
        dummyNodes.push(node8.clone());
        dummyNodes.push(node9.clone());
        dummyNodes.push(node10.clone());

  
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();


        for i in 0..20 {
            let claims = 
        }

        //let addr: String = "0x0000000000000000000000000000000000000000".to_string();
        let claim1: Claim = Claim::new(node1.pubkey.clone(), addr.clone(), 1);
        let claim2: Claim = Claim::new(node2.pubkey.clone(), addr.clone(), 2);
        let claim3: Claim = Claim::new(node3.pubkey.clone(), addr.clone(), 3);
        let claim4: Claim = Claim::new(node4.pubkey.clone(), addr.clone(), 4);
        let claim5: Claim = Claim::new(node5.pubkey.clone(), addr.clone(), 5);
        let claim6: Claim = Claim::new(node6.pubkey.clone(), addr.clone(), 6);
        let claim7: Claim = Claim::new(node7.pubkey.clone(), addr.clone(), 7);
        let claim8: Claim = Claim::new(node8.pubkey.clone(), addr.clone(), 8);
        let claim9: Claim = Claim::new(node9.pubkey.clone(), addr.clone(), 9);
        let claim10: Claim = Claim::new(node10.pubkey.clone(), addr.clone(), 10);
        dummyClaims.push(claim1);
        dummyClaims.push(claim2);
        dummyClaims.push(claim3);
        dummyClaims.push(claim4);
        dummyClaims.push(claim5);
        dummyClaims.push(claim6);
        dummyClaims.push(claim7);
        dummyClaims.push(claim8);
        dummyClaims.push(claim9);
        dummyClaims.push(claim10);

        let mut child_block = DummyChildBlock::new(b"one", b"two");
    
        let mut quorum: Quorum = Quorum::new();
        quorum.run_election(&child_block, dummyClaims, dummyNodes);

        assert!(quorum.masternodes.len() == 5);
        
    }


}
