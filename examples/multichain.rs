use node::test_utils::create_test_network;
use primitives::{Address, PublicKey};
use std::str::FromStr;
use telemetry::custom_subscriber::TelemetrySubscriber;

#[tokio::main]
async fn main() {
    // std::env::set_var("VRRB_ENVIRONMENT", "main");
    // std::env::set_var("VRRB_PRETTY_PRINT_LOGS", "true");
    // std::env::set_var("RUST_LOG", "error");

    // TelemetrySubscriber::init(std::io::stdout).unwrap();

    let nodes = create_test_network(8).await;

    for node in nodes.iter() {
        println!("{}", node.jsonrpc_server_address());
        let pubkey = PublicKey::from_str(&node.keypair.get_miner_public_key().to_string()).unwrap();
        let address = Address::new(pubkey);
        println!("Address: {}", address);
    }

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
