use assert_cmd::Command;
use cli::commands::node::deserialize_whitelisted_quorum_members;
use cli::commands::node::GENESIS_QUORUM_SIZE;
use primitives::{KademliaPeerId, NodeType};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use vrrb_config::QuorumMember;
use vrrb_core::keypair::Keypair;

#[tokio::test]
#[ignore = "temporarily broken because of GH actions issue"]
pub async fn cli_should_show_help_text() {
    let mut cmd = Command::cargo_bin("vrrb").unwrap();
    let help_text = r#"cli 0.0.0

USAGE:
    vrrb [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -c, --config <FILE>    Sets a custom config file
    -d, --debug            Turn debugging information on
    -h, --help             Print help information
    -V, --version          Print version information

SUBCOMMANDS:
    help           Print this message or the help of the given subcommand(s)
    node           Node management subcommands
    placeholder    Placeholder sub-command to demonstrate how to configure them
"#;

    cmd.arg("--help").assert().stdout(help_text).success();
}

#[test]
fn create_node_config_with_whitelist() {
    // serialize a vec of QuorumMember and write it to whitelist.json
    let mut map: serde_json::Map<String, serde_json::Value> =
        serde_json::Map::with_capacity(GENESIS_QUORUM_SIZE);
    let mut quorum_members: Vec<QuorumMember> = Vec::with_capacity(GENESIS_QUORUM_SIZE);
    for i in 1..=GENESIS_QUORUM_SIZE as u16 {
        let udp_port: u16 = 11000 + i;
        let raptor_port: u16 = 12000 + i;
        let kademlia_port: u16 = 13000 + i;
        let keypair = Keypair::random();
        let validator_public_key = keypair.miner_public_key_owned();

        let member = QuorumMember {
            node_id: format!("node-{}", i),
            kademlia_peer_id: KademliaPeerId::rand(),
            node_type: NodeType::Validator,
            udp_gossip_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), udp_port),
            raptorq_gossip_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), raptor_port),
            kademlia_liveness_address: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                kademlia_port,
            ),
            validator_public_key,
        };
        quorum_members.push(member);
    }
    assert_eq!(quorum_members.len(), GENESIS_QUORUM_SIZE);
    if let Some(bootstrap) = quorum_members.first_mut() {
        bootstrap.node_type = NodeType::Bootstrap;
        let bootstrap = serde_json::to_value(bootstrap.clone()).unwrap();
        map.insert("genesis-miner".into(), bootstrap);
    }
    let mut farmers: Vec<QuorumMember> = Vec::with_capacity(2);
    let mut harvesters: Vec<QuorumMember> = Vec::with_capacity(2);
    for (pos, member) in quorum_members.iter().skip(1).enumerate() {
        if pos > 1 {
            farmers.push(member.clone());
        } else {
            harvesters.push(member.clone());
        }
    }
    let farmers = serde_json::to_value(farmers).unwrap();
    map.insert("genesis-farmers".into(), farmers);
    let harvesters = serde_json::to_value(harvesters).unwrap();
    map.insert("genesis-harvesters".into(), harvesters);
    assert!(map
        .get(&"genesis-miner".to_string())
        .unwrap()
        .as_object()
        .is_some());
    assert_eq!(
        map.get(&"genesis-farmers".to_string())
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        map.get(&"genesis-harvesters".to_string())
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );

    assert!(serde_json::to_writer(
        std::fs::File::create("tests/whitelist_data/whitelist.json").expect("no such file"),
        &map,
    )
    .is_ok());

    let mut finalized_whitelist: Vec<QuorumMember> = Vec::with_capacity(GENESIS_QUORUM_SIZE);

    deserialize_whitelisted_quorum_members(
        "tests/whitelist_data/whitelist.json".into(),
        &mut finalized_whitelist,
    );
    finalized_whitelist.sort();

    assert_eq!(quorum_members, finalized_whitelist);
}
