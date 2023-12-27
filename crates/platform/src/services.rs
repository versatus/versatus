use bitmask_enum::bitmask;
use nix::sys::utsname::UtsName;
use serde::{Deserialize, Serialize};

use crate::{error::PlatformError, sys::MachineArchitecture};

/// An enum representing the service type. Compute, Storage, for example. More to come in the future.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ServiceType {
    /// A service that will accept (and execute) compute jobs
    Compute,
    /// A service that will handle web3 content-addressed persistence of binary blobs
    Storage,
    /// A service that supports the Versatus blockchain protocol(s)
    Blockchain,
}

/// A bitmask of capabilities supported by a particular service.
/// Subject to change, batteries not included.
#[bitmask]
#[bitmask_config(vec_debug)]
#[derive(Serialize, Deserialize)]
pub enum ServiceCapabilities {
    /// This compute service supports execution of WASM/WASI
    Wasi,
    /// This compute service supports execution of X86_64 code
    Amd64,
    /// This compute service supports execution of ARM64 code
    Aarch64,
    /// This compute service supports execution of RISC-V code
    Riscv,
    /// This compute service supports consensus (smart contract) jobs
    Consensus,
    /// This compute service supports Function-as-a-Service (FaaS) jobs
    Faas,
    /// This compute service supports long-running Node-JS jobs
    NodeJs,
    /// This storage service supports the IPFS web3 storage protocol
    Ipfs,
    /// This storage service's data store is on resilient storage
    Resilient,
}
impl TryFrom<UtsName> for ServiceCapabilities {
    type Error = PlatformError;
    fn try_from(value: UtsName) -> Result<Self, Self::Error> {
        let res = match MachineArchitecture::try_from(&value)? {
            MachineArchitecture::x86_64 => ServiceCapabilities::Amd64,
            MachineArchitecture::Aarch64 => ServiceCapabilities::Aarch64,
            MachineArchitecture::Riscv => ServiceCapabilities::Riscv,
        };
        Ok(res)
    }
}

/// A version number
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VersionNumber {
    major: u8,
    minor: u8,
    patch: u8,
}
impl VersionNumber {
    pub fn cargo_pkg() -> Self {
        let n = std::env!("CARGO_PKG_VERSION")
            .split('.')
            .map(|x| x.parse().unwrap_or(0))
            .collect::<Vec<u8>>();
        Self {
            major: n[0],
            minor: n[1],
            patch: n[2],
        }
    }
}
impl std::fmt::Display for VersionNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ServiceStatusResponse {
    /// Type of service (see above)
    pub service_type: ServiceType,
    /// Capabilities of this service (see above)
    pub service_capabilities: ServiceCapabilities,
    /// A string naming the implementation of this storage service (future proofing).
    pub service_implementation: String,
    /// The version number of this service
    pub service_version: VersionNumber,
    /// The current uptime (seconds.ns) of the service
    pub service_uptime: u64,
}
impl std::fmt::Debug for ServiceStatusResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
impl std::fmt::Display for ServiceStatusResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Service Status Response:
    service type:   {:?}
    capabilities:   {:?}({} bits)
    implementation: {:?}
    version:        v{}
    uptime:         {}s",
            self.service_type,
            self.service_capabilities,
            self.service_capabilities.bits(),
            (!self.service_implementation.is_empty()).then_some(Some(self.service_capabilities)),
            self.service_version,
            self.service_uptime
        )
    }
}
