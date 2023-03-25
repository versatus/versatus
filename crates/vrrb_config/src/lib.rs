mod bootstrap;
pub mod node_config;

pub use node_config::*;

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use primitives::NodeType;
    use vrrb_core::keypair::Keypair;

    use super::*;

    #[test]
    fn can_be_built_using_a_builder() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let keypair = Keypair::random();

        NodeConfigBuilder::default()
            .id(String::from("abcdefg"))
            .idx(10)
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
            .bootstrap_node_addresses(vec![addr])
            .keypair(keypair)
            .bootstrap_config(None)
            .build()
            .unwrap();
    }
}
