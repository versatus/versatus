use anyhow::{anyhow, Context, Result};
use futures::TryStreamExt;
use http::uri::Scheme;
use ipfs_api::{IpfsApi, IpfsClient, TryFromUri};
use serde_derive::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io::Cursor;
use std::net::{IpAddr, SocketAddr};
use trust_dns_resolver::proto::rr::RecordType;
use trust_dns_resolver::Resolver;

/// A structure representing a content-addressable Web3 store. Currently closely tied to IPFS
/// specifically, but could be expanded to others, such as Iroh.
pub struct Web3Store {
    client: IpfsClient,
}

/// A structure representing stats for a content-addressable Web3 store. Currently closely tied to
/// IPFS and Kubo specifically, but could be adapted to others.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Web3StoreStats {
    pub repo: Web3StoreRepoStats,
    pub bandwidth: Web3StoreBandwidthStats,
    pub bitswap: Web3StoreBitswapStats,
}

/// A struct representing stats about the backend storage repository
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Web3StoreRepoStats {
    /// Number of objects in the repository
    pub num_objects: u64,
    /// Size in bytes of the repository
    pub repo_size: u64,
    /// String representation of the path to the repository root
    pub repo_path: String,
}

/// A struct representing network bandwidth stats of the Web3 store
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Web3StoreBandwidthStats {
    /// Total bytes received.
    pub total_in: u64,
    /// Total bytes sent.
    pub total_out: u64,
    /// Data rate in.
    pub rate_in: f64,
    /// Data rate out.
    pub rate_out: f64,
}

/// A structure representing stats for bitswap (data transfer) only.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Web3StoreBitswapStats {
    /// Blocks of data received by bitswap.
    pub blocks_in: u64,
    /// Bytes of data received via bitswap
    pub data_in: u64,
    /// Blocks of data sent via bitswap.
    pub blocks_out: u64,
    /// Bytes of data sent via bitswap.
    pub data_out: u64,
}

impl Web3Store {
    /// A constructor that uses the default configuration to connect to a local IPFS
    /// implementation's RPC service on the default port of TCP/5001.
    pub fn local() -> Result<Self> {
        Ok(Web3Store {
            client: IpfsClient::default(),
        })
    }

    /// A constructor that takes domain host name  or SRV record and resolves it to ipv4/ipv6 addresses
    /// and then use the address for RPC service on an IPFS instance
    pub fn from_hostname(addr: &str, is_srv: bool) -> Result<Self> {
        let addresses = Self::resolve_dns(addr, is_srv)?;
        let address = addresses.first().unwrap();
        let ip = address.ip();
        let port = address.port();
        Ok(Web3Store {
            client: IpfsClient::from_host_and_port(Scheme::HTTP, ip.to_string().as_str(), port)?,
        })
    }

    /// Resolves DNS records and retrieves a list of IP addresses.
    ///
    /// # Arguments
    ///
    /// * `name` - A string representing the domain or host name to resolve.
    /// * `is_srv` - A boolean indicating whether to perform an SRV record lookup.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `IpAddr` representing the resolved IP addresses.
    ///
    /// # Errors
    ///
    /// Returns an error if the resolution fails or if no addresses are found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::web3_pkg::web3_store::Web3Store;
    ///
    /// let result = Web3Store::resolve_dns("example.com", false);
    /// match result {
    ///     Ok(addresses) => {
    ///         for addr in addresses {
    ///             println!("Resolved IP Address: {}", addr);
    ///         }
    ///     }
    ///     Err(err) => {
    ///         eprintln!("Error: {}", err);
    ///     }
    /// }
    /// ```
    pub fn resolve_dns(name: &str, is_srv: bool) -> Result<Vec<SocketAddr>> {
        let resolver = Resolver::from_system_conf()?;
        let mut addresses = Vec::new();
        if is_srv {
            let lookup = resolver.lookup(name, RecordType::SRV)?;
            for record in lookup.records() {
                if let Some(srv_data) = record.data().and_then(|data| data.as_srv()) {
                    let target = srv_data.target();
                    let address_list =
                        Self::resolve_ip_addresses(&resolver, target.to_string().as_str())?;
                    for addr in address_list {
                        addresses.push(SocketAddr::new(addr, srv_data.port()));
                    }
                }
            }
        } else {
            let address_list = Self::resolve_ip_addresses(&resolver, name)?;
            for addr in address_list {
                addresses.push(SocketAddr::new(addr, 5001));
            }
        }
        if addresses.is_empty() {
            return Err(anyhow::Error::msg("No addresses found"));
        }
        Ok(addresses)
    }

    fn resolve_ip_addresses(resolver: &Resolver, target: &str) -> Result<Vec<IpAddr>> {
        let mut addresses = Vec::new();
        let ip_address = resolver
            .lookup_ip(target)
            .context("Failed to resolve DNS to IP address")?;
        for addr in ip_address.iter() {
            addresses.push(addr)
        }
        if addresses.is_empty() {
            let ip4_address = resolver
                .lookup(target, RecordType::A)
                .context("Failed to resolve DNS to IPv4 address")?;
            let ip6_address = resolver
                .lookup(target, RecordType::AAAA)
                .context("Failed to resolve DNS to IPv6 address")?;
            let ip4_records = ip4_address.records();
            for record in ip4_records {
                if let Some(ip4_data) = record.data().and_then(|data| data.as_a()) {
                    addresses.push(IpAddr::V4(ip4_data.0));
                }
            }
            let ip6_records = ip6_address.records();
            for record in ip6_records {
                if let Some(ip6_data) = record.data().and_then(|data| data.as_aaaa()) {
                    addresses.push(IpAddr::V6(ip6_data.0));
                }
            }
        }
        Ok(addresses)
    }

    /// A constructor that takes a multiaddr string (eg, "/ip4/127.0.0.1/tcp/5001") to connect to
    /// the RPC service on an IPFS instance.
    pub fn from_multiaddr(addr: &str) -> Result<Self> {
        Ok(Web3Store {
            client: IpfsClient::from_multiaddr_str(addr)?,
        })
    }

    /// A method to take a vector of bytes and write them as a DAG to IPFS. It is expected that the
    /// bytes are either in DAG-JSON or DAG-CBOR by the underlying protocol.
    /// On success, returns a string representation of the CID of the object written.
    pub async fn write_dag(&self, data: Vec<u8>) -> Result<String> {
        let curs = Cursor::new(data);
        let cid = self.client.dag_put(curs).await?;
        Ok(cid.cid.cid_string)
    }

    /// A method to take a vector of bytes and write them to IPFS as a DAG-PB object. This is the
    /// IPFS file interface and the data is treated as opaque, but will be split into DAG blocks.
    /// On success, returns a string representation of the CID of the object written.
    pub async fn write_object(&self, data: Vec<u8>) -> Result<String> {
        let curs = Cursor::new(data);
        let cid = self.client.add(curs).await?;
        Ok(cid.hash)
    }

    /// A method to retrieve a DAG object by CID from the web3 datastore. Returns it in DAG-JSON
    /// format.
    pub async fn read_dag(&self, cid: &str) -> Result<Vec<u8>> {
        let ret = self
            .client
            .dag_get(cid)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await?;
        Ok(ret)
    }

    /// A method to retrieve an unstructured object from the web3 store by CID
    pub async fn read_object(&self, cid: &str) -> Result<Vec<u8>> {
        let ret = self
            .client
            .cat(cid)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await?;
        Ok(ret)
    }

    /// Pins the object
    pub async fn pin_object(&self, cid: &str, recursive: bool) -> Result<Vec<String>> {
        let res = self.client.pin_add(cid, recursive).await?;
        Ok(res.pins)
    }

    /// Checks if object is pinned
    pub async fn is_pinned(&self, cid: &str) -> Result<()> {
        let res = self.client.pin_ls(Some(cid), None).await?;

        if res.keys.is_empty() {
            return Err(anyhow!("The CID {} is not pinned.", cid));
        }
        Ok(())
    }
    /// A method to retrieve stats from the IPFS service and return them
    pub async fn stats(&self) -> Result<Web3StoreStats> {
        let repo = self.client.stats_repo().await?;
        let bw = self.client.stats_bw().await?;
        let bs = self.client.stats_bitswap().await?;

        Ok(Web3StoreStats {
            repo: Web3StoreRepoStats {
                num_objects: repo.num_objects,
                repo_size: repo.repo_size,
                repo_path: repo.repo_path,
            },
            bandwidth: Web3StoreBandwidthStats {
                total_in: bw.total_in,
                total_out: bw.total_out,
                rate_in: bw.rate_in,
                rate_out: bw.rate_out,
            },
            bitswap: Web3StoreBitswapStats {
                blocks_in: bs.blocks_received,
                blocks_out: bs.blocks_sent,
                data_in: bs.data_received,
                data_out: bs.data_sent,
            },
        })
    }
}
