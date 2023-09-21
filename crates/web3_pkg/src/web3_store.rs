use anyhow::Result;
use futures::TryStreamExt;
use ipfs_api::{IpfsApi, IpfsClient, TryFromUri};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

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

