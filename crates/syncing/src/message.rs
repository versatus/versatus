use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    result::Result as StdResult,
};

use chrono::Utc;
use primitives::types::node::NodeType;

use crate::{context::NodeUpdateState, error::DiscovererError, MAX_CONNECTED_NODES};

pub type Result<T> = StdResult<T, DiscovererError>;

#[derive(Debug, Clone)]
pub struct NodeRouteEntry {
    pub time: i64,
    pub node_id: String,
    pub node_type: NodeType,
    pub node_state: NodeUpdateState,
    pub node_address_v4: Ipv4Addr,
    pub node_address_v6: Ipv6Addr,
    pub discovery_port: u16,
    pub cluster_ipv4: Vec<Ipv4Addr>,
    pub cluster_ipv6: Vec<Ipv6Addr>,
}

impl NodeRouteEntry {
    pub fn new(node_id: String, node_type: NodeType, discovery_port: u16) -> Self {
        Self {
            time: Utc::now().timestamp_millis(),
            node_id,
            node_type,
            node_state: NodeUpdateState::InProgress,
            node_address_v4: Ipv4Addr::LOCALHOST,
            node_address_v6: Ipv6Addr::LOCALHOST,
            discovery_port,
            cluster_ipv4: vec![],
            cluster_ipv6: vec![],
        }
    }

    pub fn time(&self) -> &i64 {
        &self.time
    }

    pub fn id(&self) -> String {
        self.node_id.clone()
    }

    pub fn ip_v4s(&self) -> &Vec<Ipv4Addr> {
        &self.cluster_ipv4
    }

    pub fn ip_v6s(&self) -> &Vec<Ipv6Addr> {
        &self.cluster_ipv6
    }

    pub fn _ip_v4(&self) -> &Ipv4Addr {
        &self.node_address_v4
    }

    pub fn _ip_v6(&self) -> &Ipv6Addr {
        &self.node_address_v6
    }

    pub fn ip(&self) -> IpAddr {
        if self.node_address_v4 != Ipv4Addr::LOCALHOST {
            IpAddr::V4(self.node_address_v4)
        } else if self.node_address_v6 != Ipv6Addr::LOCALHOST {
            IpAddr::V6(self.node_address_v6)
        } else {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        }
    }

    pub fn _node_type(&self) -> &NodeType {
        &self.node_type
    }

    pub fn _discovery_port(&self) -> u16 {
        self.discovery_port
    }

    pub fn _add_ip_v4(&mut self, addr: Ipv4Addr) -> bool {
        if self.cluster_ipv4.len() < MAX_CONNECTED_NODES {
            self.cluster_ipv4.push(addr);
            true
        } else {
            false
        }
    }

    pub fn _add_ip_v6(&mut self, addr: Ipv6Addr) -> bool {
        if self.cluster_ipv6.len() < MAX_CONNECTED_NODES {
            self.cluster_ipv6.push(addr);
            true
        } else {
            false
        }
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        let mut pos = 0;

        let time = parse_time(buf, &mut pos)?;
        let node_id = parse_node_id(buf, &mut pos)?;
        let node_type = parse_node_type(buf, &mut pos)?;
        let node_state = parse_node_state(buf, &mut pos)?;
        let node_address_v4 = parse_ipv4_address(buf, &mut pos)?;
        let node_address_v6 = parse_ipv6_address(buf, &mut pos)?;
        let discovery_port = parse_discovery_port(buf, &mut pos)?;
        let cluster_ipv4 = parse_cluster_ipv4(buf, &mut pos)?;
        let cluster_ipv6 = parse_cluster_ipv6(buf, &mut pos)?;

        Ok(Self {
            time,
            node_id,
            node_type,
            node_state,
            node_address_v4,
            node_address_v6,
            discovery_port,
            cluster_ipv4,
            cluster_ipv6,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            8  + // time
            1  + self.node_id.len() + // size+node_id
            1  + // node_type
            4  + // node ipv4
            16 + // node ipv6
            2  + // discovery_port
            2, // size + ?
        );

        bytes.extend_from_slice(&self.time.to_be_bytes());

        bytes.push(self.node_id.len() as u8);
        bytes.extend_from_slice(self.node_id.as_bytes());

        bytes.push(self.node_type as u8);

        bytes.push(self.node_state as u8);

        bytes.extend_from_slice(&self.node_address_v4.octets());

        bytes.extend_from_slice(&self.node_address_v6.octets());

        bytes.extend_from_slice(&self.discovery_port.to_be_bytes());

        bytes.push(self.cluster_ipv4.len() as u8);
        for a4 in &self.cluster_ipv4 {
            bytes.extend_from_slice(&a4.octets());
        }

        bytes.push(self.cluster_ipv6.len() as u8);
        for a6 in &self.cluster_ipv6 {
            bytes.extend_from_slice(&a6.octets());
        }

        bytes
    }
}

fn parse_time(buf: &[u8], pos: &mut usize) -> Result<i64> {
    let time = i64::from_be_bytes([
        buf[*pos],
        buf[*pos + 1],
        buf[*pos + 2],
        buf[*pos + 3],
        buf[*pos + 4],
        buf[*pos + 5],
        buf[*pos + 6],
        buf[*pos + 7],
    ]);

    *pos += 8;

    Ok(time)
}

fn parse_node_state(buf: &[u8], pos: &mut usize) -> Result<NodeUpdateState> {
    if *pos + 1 > buf.len() {
        return Err(DiscovererError::Deserialize(
            "Distorted UDP packet".to_string(),
        ));
    }

    let node_state = match buf[*pos] {
        0 => NodeUpdateState::UpToDate,
        1 => NodeUpdateState::InProgress,
        2 => NodeUpdateState::Invalid,
        unknown_type => {
            return Err(DiscovererError::Deserialize(format!(
                "Distorted UDP packet, unknown node type : {}",
                unknown_type
            )))
        },
    };
    *pos += 1;

    Ok(node_state)
}

fn parse_node_type(buf: &[u8], pos: &mut usize) -> Result<NodeType> {
    if *pos + 1 > buf.len() {
        return Err(DiscovererError::Deserialize(
            "Distorted UDP packet".to_string(),
        ));
    }

    let node_type = match buf[*pos] {
        0 => NodeType::Full,
        1 => NodeType::Light,
        2 => NodeType::Archive,
        3 => NodeType::Miner,
        4 => NodeType::Bootstrap,
        5 => NodeType::Validator,
        6 => NodeType::MasterNode,
        unknown_type => {
            return Err(DiscovererError::Deserialize(format!(
                "Distorted UDP packet, unknown node type : {}",
                unknown_type
            )))
        },
    };
    *pos += 1;

    Ok(node_type)
}

fn parse_node_id(buf: &[u8], pos: &mut usize) -> Result<String> {
    let node_id_size = usize::from(buf[*pos]);
    *pos += 1;

    let node_id = match String::from_utf8(buf[*pos..*pos + node_id_size].to_vec()) {
        Ok(node_id) => node_id,
        Err(e) => {
            return Err(DiscovererError::Deserialize(format!(
                "Distorted UDP packet : {}",
                e
            )));
        },
    };
    *pos += node_id_size;

    Ok(node_id)
}

fn parse_discovery_port(buf: &[u8], pos: &mut usize) -> Result<u16> {
    if *pos + 2 > buf.len() {
        return Err(DiscovererError::Deserialize(
            "Distorted UDP packet".to_string(),
        ));
    }

    let service_port = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]);
    *pos += 2;

    Ok(service_port)
}

fn parse_ipv4_address(buf: &[u8], pos: &mut usize) -> Result<Ipv4Addr> {
    let isize = 4;

    if *pos + isize > buf.len() {
        return Err(DiscovererError::Deserialize(
            "Distorted UDP packet".to_string(),
        ));
    }

    let ipv4 = Ipv4Addr::new(buf[*pos], buf[*pos + 1], buf[*pos + 2], buf[*pos + 3]);

    *pos += isize;

    Ok(ipv4)
}

fn parse_ipv6_address(buf: &[u8], pos: &mut usize) -> Result<Ipv6Addr> {
    let isize = 16;

    if *pos + isize > buf.len() {
        return Err(DiscovererError::Deserialize(
            "Distorted UDP packet".to_string(),
        ));
    }

    let ipv6 = Ipv6Addr::new(
        u16::from_be_bytes([buf[*pos], buf[*pos + 1]]),
        u16::from_be_bytes([buf[*pos + 2], buf[*pos + 3]]),
        u16::from_be_bytes([buf[*pos + 4], buf[*pos + 5]]),
        u16::from_be_bytes([buf[*pos + 6], buf[*pos + 7]]),
        u16::from_be_bytes([buf[*pos + 8], buf[*pos + 9]]),
        u16::from_be_bytes([buf[*pos + 10], buf[*pos + 11]]),
        u16::from_be_bytes([buf[*pos + 12], buf[*pos + 13]]),
        u16::from_be_bytes([buf[*pos + 14], buf[*pos + 15]]),
    );

    *pos += isize;

    Ok(ipv6)
}

fn parse_cluster_ipv4(buf: &[u8], pos: &mut usize) -> Result<Vec<Ipv4Addr>> {
    if *pos + 1 > buf.len() {
        return Err(DiscovererError::Deserialize(
            "Distorted UDP packet".to_string(),
        ));
    }

    let ipv4_addrs_count = usize::from(buf[*pos]);
    *pos += 1;

    let mut ipv4_addrs = Vec::with_capacity(ipv4_addrs_count);

    for _ in 0..ipv4_addrs_count {
        if *pos + 4 > buf.len() {
            return Err(DiscovererError::Deserialize(
                "Distorted UDP packet".to_string(),
            ));
        }

        let ip = Ipv4Addr::new(buf[*pos], buf[*pos + 1], buf[*pos + 2], buf[*pos + 3]);

        ipv4_addrs.push(ip);
        *pos += 4;
    }

    Ok(ipv4_addrs)
}

fn parse_cluster_ipv6(buf: &[u8], pos: &mut usize) -> Result<Vec<Ipv6Addr>> {
    if *pos + 1 > buf.len() {
        return Err(DiscovererError::Deserialize(
            "Distorted UDP packet".to_string(),
        ));
    }

    let cluster_ipv6_count = usize::from(buf[*pos]);
    *pos += 1;

    let mut cluster_ipv6 = Vec::with_capacity(cluster_ipv6_count);

    for _ in 0..cluster_ipv6_count {
        if *pos + 4 > buf.len() {
            return Err(DiscovererError::Deserialize(
                "Distorted UDP packet".to_string(),
            ));
        }

        let ipv6 = Ipv6Addr::new(
            u16::from_be_bytes([buf[*pos], buf[*pos + 1]]),
            u16::from_be_bytes([buf[*pos + 2], buf[*pos + 3]]),
            u16::from_be_bytes([buf[*pos + 4], buf[*pos + 5]]),
            u16::from_be_bytes([buf[*pos + 6], buf[*pos + 7]]),
            u16::from_be_bytes([buf[*pos + 8], buf[*pos + 9]]),
            u16::from_be_bytes([buf[*pos + 10], buf[*pos + 11]]),
            u16::from_be_bytes([buf[*pos + 12], buf[*pos + 13]]),
            u16::from_be_bytes([buf[*pos + 14], buf[*pos + 15]]),
        );

        cluster_ipv6.push(ipv6);
        *pos += 16;
    }

    Ok(cluster_ipv6)
}
