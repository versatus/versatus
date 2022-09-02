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
    use format_bytes::format_bytes;

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
        (0..20).for_each(
            |i| {
                let msg = format_bytes!(b"node{}", &i);
                let node: DummyNode = DummyNode::new(&msg);
                dummyNodes.push(node.clone());
            }
        );
        
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();

        /*
        let foo = vec![1, 35, 64, 36, 26];
for (i, item) in foo.iter().enumerate() {
    println!("The {}th item is {}", i+1, item);
}
 */
        for (i, node) in 
        
        dummyNodes.iter().for_each(
            |node| {
                let claim: Claim = Claim::new(node.pubkey.clone(), addr.clone(), i);
                dummyClaims.push(claim);
            }
        )
        
        //let my_vec: Vec<u64> = (0..10).collect(); 
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
