#[tokio::test]
async fn vrrbdb_should_update_with_new_block() {
    // setup necessary components to conduct test,
    // including VrrbDb instance & DAG instance.
    // DAG instance should already have 1 round under its
    // belt, i.e. a GenesisBlock, at least 1 ProposalBlock and
    // at least 1 Convergence block.
    // Provide the StateModule with the necessary configuration
    // including a copy of the VrrbDb and the DAG
    // Provide the BlockHash for the ConvergenceBlock and call the
    // state_module.update_state(); method passing the ConvergenceBlock
    // hash into it.
    // Check the VrrbDb instance to ensure that the transactions and
    // claims in the PropsalBlock(s)/ConvergenceBlock are reflected
    // in the db, including in the StateStore, ClaimStore and
    // TransactionStore
    todo!();
}
