// Tests for request stats
use crate::request_stats::RequestStats;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn req_stats_ok() {
    let mut stats = RequestStats::new("Test".to_string(), "req_stats_ok".to_string())
        .expect("Failed to create new stats object");
    stats.start("1sec".to_string()).expect("Failed to start");
    sleep(Duration::from_secs(1));
    stats.stop("1sec".to_string()).expect("Failed to stop");
    stats.start("3sec".to_string()).expect("Failed to start");
    sleep(Duration::from_secs(3));
    stats.stop("3sec".to_string()).expect("Failed to stop");
}
