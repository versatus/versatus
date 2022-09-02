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
        (0..3).for_each(
            |i| {
                let msg = format_bytes!(b"node{}", &i);
                let node: DummyNode = DummyNode::new(&msg);
                dummyNodes.push(node.clone());
            }
        );
        
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();

        for (i, node) in dummyNodes.iter().enumerate(){
            let claim: Claim = Claim::new(node.pubkey.clone(), addr.clone(), i as u128);
            dummyClaims.push(claim);
        }
        

        let child_block = DummyChildBlock::new(b"one", b"two");
        let mut quorum: Quorum = Quorum::new();

        assert!(quorum.run_election(&child_block, dummyClaims, dummyNodes).is_err());
    }

    #[test]
    fn invalid_block_height() {
        let mut dummyNodes: Vec<DummyNode> = Vec::new();
        (0..10).for_each(
            |i| {
                let msg = format_bytes!(b"node{}", &i);
                let node: DummyNode = DummyNode::new(&msg);
                dummyNodes.push(node.clone());
            }
        );
        
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();

        for (i, node) in dummyNodes.iter().enumerate(){
            let claim: Claim = Claim::new(node.pubkey.clone(), addr.clone(), i as u128);
            dummyClaims.push(claim);
        }
        

        let mut child_block = DummyChildBlock::new(b"one", b"two");
        child_block.set_block_height(0);
        let mut quorum: Quorum = Quorum::new();
        assert!(quorum.run_election(&child_block, dummyClaims, dummyNodes).is_err());
        
    }

    #[test]
    fn invalid_block_timestamp() {
        let mut dummyNodes: Vec<DummyNode> = Vec::new();
        (0..10).for_each(
            |i| {
                let msg = format_bytes!(b"node{}", &i);
                let node: DummyNode = DummyNode::new(&msg);
                dummyNodes.push(node.clone());
            }
        );
        
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();

        for (i, node) in dummyNodes.iter().enumerate(){
            let claim: Claim = Claim::new(node.pubkey.clone(), addr.clone(), i as u128);
            dummyClaims.push(claim);
        }
        

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

        for (i, node) in dummyNodes.iter().enumerate(){
            let claim: Claim = Claim::new(node.pubkey.clone(), addr.clone(), i as u128);
            dummyClaims.push(claim);
        }
        
        let mut child_block = DummyChildBlock::new(b"one", b"two");
    
        let mut quorum: Quorum = Quorum::new();
        
        quorum.run_election(&child_block, dummyClaims, dummyNodes);

        assert!(quorum.masternodes.len() >= 5);
        
    } 

    /* 
    #[test]
    fn elect_quorum_nonced_up() {
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

        for (i, node) in dummyNodes.iter().enumerate(){
            let claim: Claim = Claim::new(node.pubkey.clone(), addr.clone(), i as u128);
            dummyClaims.push(claim);
        }
        
        let mut child_block = DummyChildBlock::new(b"one", b"two");
    
        let mut quorum: Quorum = Quorum::new();
        let newClaims = nonce_up_claims(&mut dummyClaims);


        quorum.run_election(&child_block, dummyClaims, dummyNodes);

        assert!(quorum.masternodes.len() >= 5);
        
    } 

    */

}
