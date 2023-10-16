mod bootstrap;
pub mod bootstrap_quorum;
mod node_config;
pub mod result;
pub mod test_utils;
pub mod threshold_config;

pub use bootstrap::*;
pub use bootstrap_quorum::*;
pub use node_config::*;
pub use result::*;
pub use test_utils::*;
pub use threshold_config::*;

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use crate::{test_utils::*, ThresholdConfig};
    use primitives::NodeType;
    use vrrb_core::keypair::Keypair;

    use super::*;

    #[test]
    fn can_be_built_using_a_builder() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let keypair = Keypair::random();

        NodeConfigBuilder::default()
            .id(String::from("abcdefg"))
            .data_dir("mock_path".into())
            .db_path("mock_path".into())
            .raptorq_gossip_address(addr)
            .udp_gossip_address(addr)
            .http_api_address(addr)
            .jsonrpc_server_address(addr)
            .http_api_title(String::from("mock title"))
            .http_api_version(String::from("1.0"))
            .http_api_shutdown_timeout(None)
            .node_type(NodeType::Full)
            .kademlia_liveness_address(addr)
            .kademlia_peer_id(None)
            .rendezvous_local_address(addr)
            .rendezvous_server_address(addr)
            .public_ip_address(addr)
            .keypair(keypair)
            .bootstrap_config(None)
            .threshold_config(ThresholdConfig::default())
            .bootstrap_quorum_config(None)
            .quorum_config(None)
            .whitelisted_nodes(vec![])
            .build()
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn successful_validate_invalid_threshold_config() {
        let invalid_config = invalid_threshold_config();
        invalid_config.validate().unwrap();
    }

    #[test]
    fn successful_validate_valid_threshold_config() {
        let valid_config = valid_threshold_config();
        valid_config.validate().unwrap();
    }
}
