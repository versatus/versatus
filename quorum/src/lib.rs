pub  mod election;
pub mod quorum;
pub mod dummyNode;

#[cfg(test)]
mod tests {
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
        dummyNodes.push(node1);
        dummyNodes.push(node2);
        dummyNodes.push(node3);
  
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();
        let claim1: Claim = Claim::new(node1.pubkey, addr, 1);
        let claim2: Claim = Claim::new(node2.pubkey, addr, 2);
        let claim3: Claim = Claim::new(node3.pubkey, addr, 3);
        dummyClaims.push(claim1);
        dummyClaims.push(claim2);
        dummyClaims.push(claim3);

        assert()




  
    }

    
}

