use node::test_utils::create_test_network;
use primitives::Address;
use telemetry::custom_subscriber::TelemetrySubscriber;
use vrrb_rpc::rpc::{api::RpcApiClient, client::create_client};
use wallet::v2::{Wallet, WalletConfig};
// use wallet::v2::{Wallet, WalletConfig};

#[tokio::main]
async fn main() {
    // std::env::set_var("VRRB_ENVIRONMENT", "main");
    // std::env::set_var("VRRB_PRETTY_PRINT_LOGS", "true");
    std::env::set_var("RUST_LOG", "info");

    // TelemetrySubscriber::init(std::io::stdout).unwrap();

    let mut nodes = create_test_network(8).await;

    let rpc_server_address = nodes.get(2).unwrap().config().jsonrpc_server_address;
    // let rpc_client = create_client(rpc_server_address).await.unwrap();
    dbg!(rpc_server_address);

    // let wal_config = WalletConfig {
    //     rpc_server_address,
    //     ..Default::default()
    // };
    //
    // let mut wal = Wallet::new(wal_config).await.unwrap();
    // let res = wal.list_transactions(vec![]).await;
    //
    // let res = wal.get_mempool().await.unwrap();
    //
    // dbg!(res);
    //

    tokio::signal::ctrl_c().await.unwrap();

    let mut node = nodes.get_mut(3).unwrap();
    let node_id = node.id();
    let rpc_server_address = node.config().jsonrpc_server_address;
    let rpc_client = create_client(rpc_server_address).await.unwrap();
    let res = rpc_client.get_full_state().await;
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

        dbg!(&wal_res.to_string());

        // for (addr, acc) in res.iter() {
        //     dbg!(&addr.to_string());
        // }
        //
        // let res = serde_json::to_string_pretty(&res).unwrap();
        // println!();
        // println!("state of {node_id}: {res}");
        println!();
    }

    // for node in nodes.iter_mut() {
    //     let node_id = node.id();
    //     let rpc_server_address = node.config().jsonrpc_server_address;
    //     let rpc_client = create_client(rpc_server_address).await.unwrap();
    //
    //     let res = rpc_client.get_full_state().await;
    //     if let Ok(res) = res {
    //         println!();
    //         for (addr, acc) in res.iter() {
    //             dbg!(&addr.to_string());
    //         }
    //         // let res = serde_json::to_string_pretty(&res).unwrap();
    //         // println!();
    //         // println!("state of {node_id}: {res}");
    //         println!();
    //     } else {
    //         println!();
    //         println!("node {node_id} has no accounts in state");
    //         println!();
    //     }
    // }

    for node in nodes {
        // let node_id = node.id();
        // let rpc_server_address = node.config().jsonrpc_server_address;
        // let rpc_client = create_client(rpc_server_address).await.unwrap();
        //
        // let res = rpc_client.get_full_state().await;
        // if let Ok(res) = res {
        //     let res = serde_json::to_string_pretty(&res).unwrap();
        //     println!();
        //     // println!("state of {node_id}: {res}");
        //     println!();
        // } else {
        //     println!();
        //     println!("node {node_id} has no accounts in state");
        //     println!();
        // }

        // println!();
        println!(
            "shutting down node {} type {:?}",
            node.id(),
            node.node_type()
        );
        // println!();

        node.stop().await.unwrap();
    }
}
