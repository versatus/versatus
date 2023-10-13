use platform::platform_stats::CgroupStats;

fn main() {
    let stats = CgroupStats::new().unwrap();
    println!("Control Group: {:?}", stats);
}
