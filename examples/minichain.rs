use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use node::{
    test_utils::{self, send_data_over_quic,generate_nodes_pattern},
    Node,
};
use telemetry::TelemetrySubscriber;

#[tokio::main]
async fn main() {
    // TelemetrySubscriber::init(std::io::stdout).unwrap();

    let mut nodes = vec![];

    let mut config = node::test_utils::create_mock_full_node_config();
    config.id = String::from("node-0");

    let node_0 = Node::start(&config).await.unwrap();

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let mut bootstrap_node_config = vrrb_config::BootstrapConfig {
        id: node_0.kademlia_peer_id(),
        udp_gossip_addr: addr,
        raptorq_gossip_addr: addr,
        kademlia_liveness_addr: addr,
    };

    bootstrap_node_config.udp_gossip_addr = node_0.udp_gossip_address();
    bootstrap_node_config.raptorq_gossip_addr = node_0.raprtorq_gossip_address();
    bootstrap_node_config.kademlia_liveness_addr = node_0.kademlia_liveness_address();

    nodes.push(node_0);
    let nodes_types=generate_nodes_pattern(8);
    for i in 1..8 {
        let mut config = node::test_utils::create_mock_full_node_config();
        config.id = format!("node-{}", i);
        config.bootstrap_config = Some(bootstrap_node_config.clone());
        config.node_type=nodes_types.get(i).unwrap().clone();
        let node = Node::start(&config).await.unwrap();
        nodes.push(node);
    }


    for node in nodes {
        println!("shutting down node {} type {:?}", node.id(),node.node_type());
        node.stop().await.unwrap();
    }
}

