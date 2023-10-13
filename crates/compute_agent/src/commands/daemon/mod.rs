use anyhow::Result;
use clap::Parser;
use service_config::ServiceConfig;
use hyper::{
    header::CONTENT_TYPE,
    service::{make_service_fn, service_fn},
    Request,
    Response,
    Body,
    Server
};
use prometheus::{TextEncoder, Encoder, Counter, register_counter, opts, labels};
use lazy_static::lazy_static;
use platform::platform_stats::CgroupStats;

/// Structure representing command line options to the daemon subcommand
#[derive(Parser, Debug)]
pub struct DaemonOpts {
}

// Define some initial counters to expose from the platform crate, plus some metadata for 
// the benefit of Prometheus and those consuming its timeseries data.
lazy_static! {
    static ref CPU_TOTAL_USEC: Counter = register_counter!(opts!(
            "cpu_total_usec",
            "CPU time used in usec total",
            labels! { "service" => "compute", "source" => "versatus" }
    )).unwrap();
    static ref CPU_USER_USEC: Counter = register_counter!(opts!(
            "cpu_user_usec",
            "CPU time used for userspace in usec",
            labels! { "service" => "compute", "source" => "versatus" }
    )).unwrap();
    static ref CPU_SYSTEM_USEC: Counter = register_counter!(opts!(
            "cpu_system_usec",
            "CPU time used for kernel in usec",
            labels! { "service" => "compute", "source" => "versatus" }
    )).unwrap();
    static ref MEM_ANON_BYTES: Counter = register_counter!(opts!(
            "mem_anon_bytes",
            "Anonymous memory used in bytes",
            labels! { "service" => "compute", "source" => "versatus" }
    )).unwrap();
    static ref MEM_FILE_BYTES: Counter = register_counter!(opts!(
            "mem_file_bytes",
            "File-backed memory used in bytes",
            labels! { "service" => "compute", "source" => "versatus" }
    )).unwrap();
    static ref MEM_SOCK_BYTES: Counter = register_counter!(opts!(
            "mem_sock_bytes",
            "Socket memory used in bytes",
            labels! { "service" => "compute", "source" => "versatus" }
    )).unwrap();
}

/// Serve Prometheus exporter requests
async fn serve_req(_req: Request<Body>) -> Result<Response<Body>, anyhow::Error> {
    let encoder = TextEncoder::new();

    // Collect stats from the platform
    let stats = CgroupStats::new()?;

    // Set total usec metric
    CPU_TOTAL_USEC.reset();
    // Lossy conversion to f64....
    CPU_TOTAL_USEC.inc_by(stats.cpu.cpu_total_usec as f64);
    CPU_USER_USEC.reset();
    CPU_USER_USEC.inc_by(stats.cpu.cpu_user_usec as f64);
    CPU_SYSTEM_USEC.reset();
    CPU_SYSTEM_USEC.inc_by(stats.cpu.cpu_system_usec as f64);
    MEM_ANON_BYTES.reset();
    MEM_ANON_BYTES.inc_by(stats.mem.mem_anon_bytes as f64);
    MEM_FILE_BYTES.reset();
    MEM_FILE_BYTES.inc_by(stats.mem.mem_file_bytes as f64);
    MEM_SOCK_BYTES.reset();
    MEM_SOCK_BYTES.inc_by(stats.mem.mem_sock_bytes as f64);

    let metrics = prometheus::gather();
    let mut buffer = vec![];

    encoder.encode(&metrics, &mut buffer)?;

    let response = Response::builder()
        .status(200)
        .header(CONTENT_TYPE, encoder.format_type())
        .body(Body::from(buffer))?;

    Ok(response)
}

/// Start the Compute Agent Daemon
pub async fn run(_opts: &DaemonOpts, config: &ServiceConfig) -> Result<()> {
    // XXX: This is where we should start the RPC server listener and process incoming requests
    // using the service name and service config provided in the global command line options.

    // In the interim, start a stub of a Prometheus exporter. Later we'll fill this with valid
    // metrics.
    let addr = format!("{}:{}", config.exporter_address, config.exporter_port)
        .parse()
        .expect("Invalid address/port for Prometheus Exporter service");
    // Execute this in the foreground til we have other work to do. Later, it can
    // end up in a long-lived thread.
    Server::bind(&addr).serve(
        make_service_fn(|_| async {
            Ok::<_, anyhow::Error>(service_fn(serve_req))
        })
    ).await?;

    Ok(())
}
