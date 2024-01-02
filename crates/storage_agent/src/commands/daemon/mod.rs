use anyhow::Result;
use clap::Parser;
use internal_rpc::server::InternalRpcServer;
use metric_exporter::metric_factory::PrometheusFactory;
use platform::services::ServiceType;
use prometheus::labels;
use service_config::ServiceConfig;
use telemetry::info;
use tokio::{signal, spawn};
use tokio_util::sync::CancellationToken;

pub const SERVICE_NAME: &str = "storage";
pub const SERVICE_SOURCE: &str = "versatus";

/// Structure representing command line options to the daemon subcommand
#[derive(Parser, Debug)]
pub struct DaemonOpts;

/// Start the Storage Agent Daemon
pub async fn run(_opts: &DaemonOpts, config: &ServiceConfig) -> Result<()> {
    // XXX: This is where we should start the RPC server listener and process incoming requests
    // using the service name and service config provided in the global command line options.
    let (_server_handle, _server_local_addr) =
        InternalRpcServer::start(config, ServiceType::Storage).await?;

    let base_labels = labels! {
                "service".to_string() => SERVICE_NAME.to_string(),
                "source".to_string() => SERVICE_SOURCE.to_string(),
    };
    let port = config
        .exporter_port
        .parse::<u16>()
        .expect("Invalid port for Prometheus Exporter service");
    let factory = PrometheusFactory::new(
        config.exporter_address.clone(),
        port,
        true,
        base_labels,
        config.tls_ca_cert_file.clone(),
        config.tls_private_key_file.clone(),
        CancellationToken::new(),
    )
    .expect("Failed to construct prometheus exporter service");

    let mut sighup_receiver = signal::unix::signal(signal::unix::SignalKind::hangup())
        .expect("Failed to construct SIGHUP receiver");

    let (sender, receiver) = tokio::sync::mpsc::channel::<()>(100);
    let server = factory.serve(receiver);
    spawn(async move {
        while (sighup_receiver.recv().await).is_some() {
            // Do something when a SIGHUP signal is received
            if (sender.send(()).await).is_err() {
                // Handle the error if sending fails
                info!("Failed to send signal");
                break; // Break out of the loop if sending fails
            } else {
                info!("Sending signal to reload config")
            }
        }
    });

    // Await the server
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}
