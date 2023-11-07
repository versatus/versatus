use node::test_utils::create_test_network;

#[tokio::main]
async fn main() {
    std::env::set_var("VRRB_ENVIRONMENT", "main");
    std::env::set_var("VRRB_PRETTY_PRINT_LOGS", "true");
    std::env::set_var("RUST_LOG", "error");

    let nodes = create_test_network(8).await;

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
