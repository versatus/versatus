use std::net::{
    IpAddr,
    Ipv4Addr,
};

use serde::{
    Deserialize
};

use uuid::Uuid;

use crate::message::NodeType;

/// Application config
/// TODO: To be integrated with the general Node configuration strategy.
#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AppConfig {
    pub node_type: NodeType,
    pub node_id: String,
    pub node_name: String,
    pub is_node_origin: bool,                   // Is this one of the core origin nodes, storing ultimate source of truth ?
    pub file_path_localstate: String,
    pub discovery_bind_local_address: String,   // "0.0.0.0" or specific interface 192.168.1.10
    pub discovery_broadcast_address: String,    // "255.255.255.255" or specific sub network 192.168.255.255
    pub discovery_port: u16,                    // common discovery port
    pub broker_local_ip: IpAddr,                // broker local ip
    pub broker_port: u16,                       // broker port
}

/// Default values for the application config.
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            node_type: NodeType::Archive,
            node_id: Uuid::new_v4().to_string(),
            node_name: String::from("MASTER"),
            is_node_origin: true,
            file_path_localstate: String::from("./testnet.db"),
            discovery_bind_local_address: IpAddr::V4(Ipv4Addr::UNSPECIFIED).to_string(),
            discovery_broadcast_address: IpAddr::V4(Ipv4Addr::BROADCAST).to_string(),
            discovery_port: 5531,
            broker_local_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            broker_port: 10001
        }
    }
}
