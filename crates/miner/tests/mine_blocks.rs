use miner::Miner;

#[test]
pub fn can_mine_genesis_block() {
    let miner = Miner::new();
    let genesis_block = miner.genesis().unwrap();
    dbg!(genesis_block);
}
