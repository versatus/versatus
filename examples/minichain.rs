use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use node::{
    test_utils::{
        self,
        create_mock_transaction_args,
        create_node_rpc_client,
        generate_nodes_pattern,
        send_data_over_quic,
    },
    Node,
};
use primitives::{KademliaPeerId, NodeType};
use telemetry::TelemetrySubscriber;
use vrrb_config::{
    BootstrapQuorumConfig,
    QuorumKind,
    QuorumMember,
    QuorumMembership,
    QuorumMembershipConfig,
};
use vrrb_rpc::rpc::api::RpcApiClient;

#[tokio::main]
async fn main() {
    std::env::set_var("VRRB_ENVIRONMENT", "main");
    std::env::set_var("VRRB_PRETTY_PRINT_LOGS", "true");

    // TelemetrySubscriber::init(std::io::stdout).unwrap();

    let mut nodes = vec![];

    let mut quorum_members = vec![];

    for i in 1..=8 {
        let kademlia_port: u16 = 10230 + i;
        let udp_port: u16 = 11000 + i;
        let raptor_port: u16 = 12000 + i;
        let member = QuorumMembership {
            member: QuorumMember {
                node_id: format!("node-{}", i),
                kademlia_peer_id: KademliaPeerId::rand(),
                node_type: NodeType::Validator,
                udp_gossip_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), udp_port),
                raptorq_gossip_address: SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::LOCALHOST),
                    raptor_port,
                ),
                kademlia_liveness_address: SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::LOCALHOST),
                    kademlia_port,
                ),
            },
            quorum_kind: QuorumKind::Farmer,
        };

        quorum_members.push(member)
    }

    let bootstrap_quorum_config = BootstrapQuorumConfig {
        membership_config: QuorumMembershipConfig {
            quorum_members: quorum_members.clone(),
        },
        genesis_transaction_threshold: 3,
    };

    let mut config = node::test_utils::create_mock_full_node_config();
    config.id = String::from("node-0");

    config.bootstrap_quorum_config = Some(bootstrap_quorum_config.clone());

    let node_0 = Node::start(&config).await.unwrap();

    let node_0_rpc_addr = node_0.jsonrpc_server_address();

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

    for i in 1..=6 {
        let mut config = node::test_utils::create_mock_full_node_config();

        let quorum_config = quorum_members.get(i - 1).unwrap();

        config.id = format!("node-{}", i);
        config.bootstrap_config = Some(bootstrap_node_config.clone());
        config.node_type = NodeType::Validator;
        config.kademlia_liveness_address = quorum_config.member.kademlia_liveness_address;
        config.raptorq_gossip_address = quorum_config.member.raptorq_gossip_address;
        config.udp_gossip_address = quorum_config.member.udp_gossip_address;
        config.kademlia_peer_id = Some(quorum_config.member.kademlia_peer_id);

        let node = Node::start(&config).await.unwrap();
        nodes.push(node);
    }

    for i in 7..=8 {
        let mut miner_config = node::test_utils::create_mock_full_node_config();

        let quorum_config = quorum_members.get(i - 1).unwrap();

        miner_config.id = format!("node-{}", i);
        miner_config.bootstrap_config = Some(bootstrap_node_config.clone());
        miner_config.node_type = NodeType::Miner;
        miner_config.kademlia_liveness_address = quorum_config.member.kademlia_liveness_address;
        miner_config.raptorq_gossip_address = quorum_config.member.raptorq_gossip_address;
        miner_config.udp_gossip_address = quorum_config.member.udp_gossip_address;
        miner_config.kademlia_peer_id = Some(quorum_config.member.kademlia_peer_id);

        let miner_node = Node::start(&miner_config).await.unwrap();
        nodes.push(miner_node);
    }

    tokio::time::sleep(Duration::from_secs(4)).await;

    // let rpc_client = create_node_rpc_client(node_0_rpc_addr).await;
    //
    // for i in 0..10 {
    //     let args = create_mock_transaction_args(i * 3);
    //
    //     rpc_client.create_txn(args).await.unwrap();
    // }

    // dbg!(rpc_client.get_full_mempool().await.unwrap().len());

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
