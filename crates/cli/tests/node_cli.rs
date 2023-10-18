use assert_cmd::prelude::OutputAssertExt;
use assert_cmd::Command;
use cli::commands::{
    node::GENESIS_QUORUM_SIZE,
    utils::{derive_kademlia_peer_id_from_node_id, deserialize_whitelisted_quorum_members},
};
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
    let mut quorum_members = create_test_genesis_quorum_member_list();

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
    )
    .unwrap();
    finalized_whitelist.sort();

    assert_eq!(quorum_members, finalized_whitelist);
}

#[test]
fn can_derive_kademlia_peer_id_from_node_id() {
    let mut quorum_members = create_test_genesis_quorum_member_list();

    quorum_members.iter_mut().for_each(|member| {
        member.kademlia_peer_id = derive_kademlia_peer_id_from_node_id(&member.node_id).unwrap()
    });

    let derived_ids = quorum_members
        .iter()
        // NOTE: kademlia-dht uses upper hex encoding to display the ids
        .map(|member| hex::encode_upper(member.kademlia_peer_id.to_key()))
        .collect::<Vec<String>>();

    let target_kademlia_peer_ids = [
        "DE2D89C8474A57C43972B36197B85CB7240A8128BB071C51C277DF0EBD91A559",
        "429D655E2613D6E1095CED64AF8E78EB8A9342E3512FCEEF6BB7196B9B4639E2",
        "A161F8F230353EC501212C38EA741E492F51C5DF5B89A0575BAAC73ADE40A4BB",
        "0579D9A36F2144432C2B4E6BAF61B93AB2063D573E0B473B7E42F74B5A8B3DEB",
        "154939C4CDE64BEB89FCB02351182CF46C58B6EE742107A07EA6D1C97889A108",
    ];

    // NOTE: peer id derivation should be deterministic
    assert_eq!(derived_ids, target_kademlia_peer_ids);
}

fn create_test_genesis_quorum_member_list() -> Vec<QuorumMember> {
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

    quorum_members
}
