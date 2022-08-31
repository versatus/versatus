let quorum_seed = Quorum::generate_quorum_seed(blockchain);
      let child_block = Blockchain::get_child_ref(blockchain);
      let mut child_block_timestamp: u128;
      let mut child_block_height: u128;

      //let child_block = Optional<Block>
      //check and provide a default error handling with if let Some
      //child block auto unwrapped by if let; 
      //if dealing w custom Result enum, 
      //Some(Binding, name you want variable to be called)

      if let Some(child_block) = child_block {
         child_block_timestamp = child_block.timestamp;
         child_block_height = child_block.height;
      } else {
         return Err(InvalidQuorum::InvalidChildBlockError);
      }
      let mut dummyNodes: Vec<DummyNode> = Vec::new();
      let node1: DummyNode = DummyNode::new(b"node1");
      let node2: DummyNode = DummyNode::new(b"node2");
      let node3: DummyNode = DummyNode::new(b"node3");

      let mut dummyClaims: Vec<Claim> = Vec::new();
      let addr: String = "0x0000000000000000000000000000000000000000".to_string();
      let claim1: Claim = Claim::new(node1.pubkey, addr, 1);
      let claim2: Claim = Claim::new(node2.pubkey, addr, 2);
      let claim3: Claim = Claim::new(node3.pubkey, addr, 3);

         //

      let masterClaims: Vec<Claim> = Quorum::get_master_claims(dummyClaims);
      let masternodes: Vec<DummyNode> = Quorum::get_quorum_nodes(
         quorum_seed, masterClaims, dummyNodes);


      //rerun when fails --> what is threshold of failure?
      //need 60% of quorum to vite, dont wanna wait until only 60% of quorum is live
      //maybe if 20% of quorum becomes faulty, we do election
      //for now, add failed field, set false, if command from external network
      //and counter of ndoe fail, check if counter meets threshold 
      //the failed to true, rerun
      Quorum{
         quorum_seed,
         masternodes,
         quorum_pk: String::new(),
         election_block_height: child_block_height,
         election_timestamp: child_block_timestamp,
      }