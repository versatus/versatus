use std::time::Duration;

use node::test_utils::create_test_network;
use telemetry::TelemetrySubscriber;

#[tokio::main]
async fn main() {
    std::env::set_var("VRRB_ENVIRONMENT", "main");
    std::env::set_var("VRRB_PRETTY_PRINT_LOGS", "true");

    TelemetrySubscriber::init(std::io::stdout).unwrap();

    let nodes = create_test_network(8).await;

    tokio::time::sleep(Duration::from_secs(3)).await;

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
