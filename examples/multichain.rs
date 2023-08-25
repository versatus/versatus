use node::test_utils::create_test_network;
// use telemetry::TelemetrySubscriber;

#[tokio::main]
async fn main() {
    std::env::set_var("VRRB_ENVIRONMENT", "main");
    std::env::set_var("VRRB_PRETTY_PRINT_LOGS", "true");
    std::env::set_var("RUST_LOG", "error");

    // TelemetrySubscriber::init(std::io::stdout).unwrap();

    let nodes = create_test_network(8).await;

    // let rpc_client = create_node_rpc_client(node_0_rpc_addr).await;
    //
    // for i in 0..10 {
    //     let args = create_mock_transaction_args(i * 3);
    //
    //     rpc_client.create_txn(args).await.unwrap();
    // }

    // dbg!(rpc_client.get_full_mempool().await.unwrap().len());

    tokio::signal::ctrl_c().await.unwrap();

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
