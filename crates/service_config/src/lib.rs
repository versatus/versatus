use anyhow::{anyhow, Result};
use serde_derive::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

/// High level wrapper struct to allow us to add things to this configuration file later that
/// aren't network service parameters. Some runtime-configuration parameters are best suited as
/// command line options, but others (especially those we will later wish to manage
/// programmatically) are better suited to an on-disk configuration file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// The services network configuration entries.
    pub services: ServiceCollectionConfig,
}

/// A structure representing the entire collection of the ServiceConfigs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceCollectionConfig {
    /// List of compute node endpoints
    pub compute: Vec<ServiceConfig>,
    /// List of blob-storage node endpoints
    pub storage: Vec<ServiceConfig>,
    /// List of blockchain/protocol node endpoints
    pub blockchain: Vec<ServiceConfig>,
}

/// A structure representing the necessary configuration items required for a network service
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceConfig {
    /// The name of this service definition
    pub name: String,
    /// The address to bind to for RPC calls
    pub rpc_address: String,
    /// The port to bind to for RPC calls
    pub rpc_port: u32,
    /// A preshared key for authenticating RPC calls
    pub pre_shared_key: String,
    /// A TLS private key for RPC transport privacy
    pub tls_private_key_file: String,
    /// A TLS public certificate for RPC transport privacy
    pub tls_public_cert_file: String,
    /// A TLS CA certificate for validating certificates
    pub tls_ca_cert_file: String,
    /// Prometheus exporter bind address
    pub exporter_address: String,
    /// Prometheus exporter bind port
    pub exporter_port: String,
}

impl Config {
    /// Given a JSON file representing a service configuration collection, return an object.
    pub fn from_file(filename: &str) -> Result<Self> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }

    /// Find a service definition by type and label
    pub fn find_service(&self, service_name: &str, service_type: &str) -> Result<ServiceConfig> {
        let collection: Vec<ServiceConfig> = match service_type {
            "compute" => self.services.compute.clone(),
            "storage" => self.services.storage.clone(),
            "blockchain" => self.services.blockchain.clone(),
            _ => return Err(anyhow!("Invalid service type: {}", service_type)),
        };

        for svc in collection.iter() {
            if svc.name == service_name {
                return Ok(svc.clone());
            }
        }
        Err(anyhow!(
            "Service {} not found as a {} service",
            service_name,
            service_type
        ))
    }
}
