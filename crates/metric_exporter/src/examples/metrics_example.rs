use metric_exporter::metric_factory::PrometheusFactory;
use prometheus::labels;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Configuration
    let port = 8080u16;
    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
    let labels = labels! {
                "service".to_string() => "compute".to_string(),
                "source".to_string() => "versatus".to_string(),
    };
    // Prometheus factory for metrics
    let factory = Arc::new(PrometheusFactory::new(port, false, HashMap::new()));

    // Metrics: Block height, Transactions per minute, CPU load, Active peers, Block finality time
    let block_height = factory
        .build_int_counter("block_height", "Current Block Height", labels.clone())
        .unwrap();
    let txn_histogram = factory
        .build_histogram("no_of_txns", "No of txns per min", labels.clone())
        .unwrap();
    let cpu_load = factory
        .build_gauge("current_cpu_load", "CPU Load", labels.clone())
        .unwrap();
    let active_peers = factory
        .build_int_gauge("active_peers", "Active Peers", labels.clone())
        .unwrap();
    let block_finality = factory
        .build_histogram("block_finality_time", "Block Finality Time", labels)
        .unwrap();

    // Simulating blockchain metrics
    let server = factory.serve();

    // Simulate block creation - Increment block height counter every 5 seconds
    thread::spawn({
        let block_height_clone = block_height.clone();
        move || {
            for _ in 0.. {
                sleep(Duration::from_secs(5));
                block_height_clone.inc();
            }
        }
    });

    // Simulate transactions - Observe transaction histogram every 10 seconds
    thread::spawn({
        let txn_histogram_clone = txn_histogram.clone();
        move || {
            for _ in 0.. {
                txn_histogram_clone.observe(200000.0);
                sleep(Duration::from_secs(10));
            }
        }
    });

    // Simulate CPU load changes - Set CPU load gauge randomly every 3 seconds
    thread::spawn({
        let cpu_load_clone = cpu_load.clone();
        move || {
            use rand::prelude::*;
            let mut rng = rand::thread_rng();
            for _ in 0.. {
                let load: f64 = rng.gen_range(0.0..100.0);
                cpu_load_clone.set(load);
                sleep(Duration::from_secs(3));
            }
        }
    });

    // Simulate active peers - Randomly change active peers count every 15 seconds
    thread::spawn({
        let active_peers_clone = active_peers.clone();
        move || {
            use rand::prelude::*;
            let mut rng = rand::thread_rng();
            for _ in 0.. {
                let peer_count: i64 = rng.gen_range(10..50); // Simulating between 10 to 50 peers as integers
                active_peers_clone.set(peer_count);
                sleep(Duration::from_secs(15));
            }
        }
    });

    // Simulate block finality time - Observe finality histogram every 30 seconds
    thread::spawn({
        let block_finality_clone = block_finality.clone();
        move || {
            for _ in 0.. {
                let finality_time: f64 = 4.5; // Simulating a constant block finality time for demo purposes
                block_finality_clone.observe(finality_time);
                sleep(Duration::from_secs(30));
            }
        }
    });

    println!("Exporter listening on http://{}", addr);

    // Await the server
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
