use std::str::FromStr;

use node::test_utils::create_test_network;
use primitives::Address;
use vrrb_rpc::rpc::{api::RpcApiClient, client::create_client};

#[tokio::main]
async fn main() {
    std::env::set_var("VRRB_ENVIRONMENT", "main");
    std::env::set_var("VRRB_PRETTY_PRINT_LOGS", "true");
    std::env::set_var("RUST_LOG", "info");

    let mut nodes = create_test_network(8).await;
    let rpc_server_address = nodes.get(4).unwrap().config().jsonrpc_server_address;

    tokio::signal::ctrl_c().await.unwrap();

    let mut node = nodes
        .iter_mut()
        .find(|node| node.read_handle().state_store_values().is_ok())
        .unwrap();

    let nodes = create_test_network(8).await;
    let node_id = node.id();
    let rpc_server_address = node.config().jsonrpc_server_address;
    let rpc_client = create_client(rpc_server_address).await.unwrap();
    let res = rpc_client.get_full_state().await;

	for node in nodes.iter() {
        println!("{}", node.jsonrpc_server_address());
        println!(
            "Prometheus Address {:?}:{}",
            node.prometheus_bind_address(),
            node.prometheus_bind_port()
        );
        let pubkey = PublicKey::from_str(&node.keypair.get_miner_public_key().to_string()).unwrap();
        let address = Address::new(pubkey);
        println!("Address: {}", address);
    }

    if let Ok(res) = res {
        println!();

        let addrs = res.keys().collect::<Vec<&Address>>();

        let from = addrs[0];
        let to = addrs[1];
        let amount = 1000;

        let wal_config = WalletConfig {
            rpc_server_address,
            ..Default::default()
        };

        let mut wal = Wallet::new(wal_config).await.unwrap();
        let now = chrono::Utc::now().timestamp();
        let wal_res = wal
            .send_transaction(to.to_owned(), amount, Default::default(), now)
            .await
            .unwrap();

        println!("{}", &wal_res.to_string());
        println!("{:?}", node.mempool_read_handle().entries());
        println!();
    } else {
        dbg!("error");
    }

    println!("press ctrl-c again to exit");

    tokio::signal::ctrl_c().await.unwrap();

    for node in nodes {
        println!();
        println!(
            "{} node {} mempool: {}, state: {}",
            node.node_type(),
            node.id(),
            node.mempool_read_handle().entries().len(),
            node.read_handle()
                .state_store_values()
                .unwrap_or_default()
                .len(),
        );
        println!("shutting down node {}", node.id(),);
        // println!();

        node.stop().await.unwrap();
    }
}
