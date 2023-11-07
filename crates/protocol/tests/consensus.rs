use node::test_utils::create_test_network;
use storage::storage_utils::remove_vrrb_data_dir;

#[tokio::test]
async fn network_can_form_genesis_quorum() {
    remove_vrrb_data_dir();
    let nodes = create_test_network(8).await;

    for node in nodes {
        println!();
        println!(
            "shutting down node {} type {:?}",
            node.id(),
            node.node_type()
        );
        println!();

        node.stop().await.unwrap();
    }
}
