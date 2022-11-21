use std::{net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub id: primitives::types::NodeId,
    pub idx: primitives::types::NodeIdx,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub gossip_address: SocketAddr,
    pub node_type: primitives::types::NodeType,
    pub bootstrap: bool,
    pub bootstrap_node_addresses: Vec<SocketAddr>,
    pub http_api_address: SocketAddr,
    pub http_api_title: String,
    pub http_api_version: String,
    pub http_api_shutdown_timeout: Option<Duration>,
}

impl NodeConfig {
    pub fn db_path(&self) -> &PathBuf {
        // TODO: refactor to Option and check if present and return configured db path
        // or default path within vrrb's data dir
        &self.db_path
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
