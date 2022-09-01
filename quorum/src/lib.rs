pub  mod election;
pub mod quorum;
pub mod dummyNode;
pub mod dummyChildBlock;

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

}
