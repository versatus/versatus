use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

use std::{
    result::Result as StdResult,
};

use crate::error::DiscovererError;

pub type Result<T> = StdResult<T, DiscovererError>;

pub const MAX_CONNECTED_NODES: usize = 8;

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum NodeType {
    Archive = 0,
    Computational = 1,
    Miner = 2,
    Validator = 3,
}

#[derive(Debug, Clone)]
pub struct NodeRouteEntry {
    pub node_id: String,
    pub node_type: NodeType,
    pub node_address_v4: Ipv4Addr,
    pub node_address_v6: Ipv6Addr,
    pub discovery_port: u16,
    pub ipv4_addresses: Vec<Ipv4Addr>,
    pub ipv6_addresses: Vec<Ipv6Addr>,
}

impl NodeRouteEntry {
    pub fn new( node_id: String,
                node_type: NodeType,
                discovery_port: u16) -> Self {

        Self {
            node_id,
            node_type,
            node_address_v4: Ipv4Addr::LOCALHOST,
            node_address_v6: Ipv6Addr::LOCALHOST,
            discovery_port,
            ipv4_addresses: vec![],
            ipv6_addresses: vec![],
        }
    }

    pub fn id(&self) -> String {
        self.node_id.clone()
    }

    pub fn ip_v4s(&self) -> &Vec<Ipv4Addr> {
        &self.ipv4_addresses
    }

    pub fn ip_v6s(&self) -> &Vec<Ipv6Addr> {
        &self.ipv6_addresses
    }

    pub fn ip_v4(&self) -> &Ipv4Addr {
        &self.node_address_v4
    }

    pub fn ip_v6(&self) -> &Ipv6Addr {
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

    pub fn discovery_port(&self) -> u16 {
        self.discovery_port
    }

    pub fn add_ip_v4(&mut self, addr: Ipv4Addr) -> bool {
        if self.ipv4_addresses.len() < MAX_CONNECTED_NODES {
            self.ipv4_addresses.push(addr);
            true
        } else {
            false
        }
    }

    pub fn add_ip_v6(&mut self, addr: Ipv6Addr) -> bool {
        if self.ipv6_addresses.len() < MAX_CONNECTED_NODES {
            self.ipv6_addresses.push(addr);
            true
        } else {
            false
        }
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self> {

        let mut pos = 0;

        let node_id = parse_node_id(buf, &mut pos)?;
        let node_type = parse_node_type(buf, &mut pos)?;
        let node_address_v4 = parse_ipv4_address(buf, &mut pos)?;
        let node_address_v6 = parse_ipv6_address(buf, &mut pos)?;
        let discovery_port = parse_discovery_port(buf, &mut pos)?;
        let ipv4_addresses = parse_ipv4_addresses(buf, &mut pos)?;
        let ipv6_addresses = parse_ipv6_addresses(buf, &mut pos)?;

        Ok(Self {
            node_id,
            node_type,
            node_address_v4,
            node_address_v6,
            discovery_port,
            ipv4_addresses,
            ipv6_addresses,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {

        let mut bytes = Vec::with_capacity(
            1 + self.node_id.len() + // size+node_id
            1 + // node_type
            4 + // node ipv4
            16 + // node ipv6
            2 + // discovery_port
            2 // size + ?
        );

        bytes.push(self.node_id.len() as u8);
        bytes.extend_from_slice(self.node_id.as_bytes());
        
        bytes.push(self.node_type as u8);

        bytes.extend_from_slice(&self.node_address_v4.octets());

        bytes.extend_from_slice(&self.node_address_v6.octets());

        bytes.extend_from_slice(&self.discovery_port.to_be_bytes());

        bytes.push(self.ipv4_addresses.len() as u8);
        for a4 in &self.ipv4_addresses {

            bytes.extend_from_slice(&a4.octets());
        }

        bytes.push(self.ipv6_addresses.len() as u8);
        for a6 in &self.ipv6_addresses {

            bytes.extend_from_slice(&a6.octets());
        }

        bytes
    }
}

fn parse_node_type(buf: &[u8], pos: &mut usize) -> Result<NodeType> {

    if *pos + 1 > buf.len() {
        return Err(DiscovererError::Deserialize(format!("Distorted UDP packet")));
    }

    let node_type = match buf[*pos] {
        0 => NodeType::Archive,
        1 => NodeType::Computational,
        2 => NodeType::Miner,
        3 => NodeType::Validator,
        unknown_type => return Err(DiscovererError::Deserialize(format!("Distorted UDP packet, unknown node type : {}", unknown_type))),
    };
    *pos += 1;
 
    Ok(node_type)
}

fn parse_node_id(
    buf: &[u8],
    pos: &mut usize,
) -> Result<String> {

    let node_id_size = usize::from(buf[*pos]);
    *pos += 1;

    let node_id = match String::from_utf8(buf[*pos..*pos + node_id_size].to_vec()) {
        Ok(node_id) => node_id,
        Err(e) => {
            return Err(DiscovererError::Deserialize(format!("Distorted UDP packet : {}", e)));
        }
    };
    *pos += node_id_size;

    Ok(node_id)
}

fn parse_discovery_port(buf: &[u8], pos: &mut usize) -> Result<u16> {

    if *pos + 2 > buf.len() {
        return Err(DiscovererError::Deserialize(format!("Distorted UDP packet")));
    }

    let service_port = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]);
    *pos += 2;

    Ok(service_port)
}

fn parse_ipv4_address(buf: &[u8], pos: &mut usize) -> Result<Ipv4Addr> {

    let isize = 4;

    if *pos + isize > buf.len() {
        return Err(DiscovererError::Deserialize(format!("Distorted UDP packet")));
    }

    let ipv4 = Ipv4Addr::new(buf[*pos], buf[*pos + 1], buf[*pos + 2], buf[*pos + 3]);

    *pos += isize;

    Ok(ipv4)
}

fn parse_ipv6_address(buf: &[u8], pos: &mut usize) -> Result<Ipv6Addr> {

    let isize = 16;

    if *pos + isize > buf.len() {
        return Err(DiscovererError::Deserialize(format!("Distorted UDP packet")));
    }

    let ipv6 = Ipv6Addr::new(
        u16::from_be_bytes([buf[*pos + 0], buf[*pos + 1]]),
        u16::from_be_bytes([buf[*pos + 2], buf[*pos + 3]]),
        u16::from_be_bytes([buf[*pos + 4], buf[*pos + 5]]),
        u16::from_be_bytes([buf[*pos + 6], buf[*pos + 7]]),
        u16::from_be_bytes([buf[*pos + 8], buf[*pos + 9]]),
        u16::from_be_bytes([buf[*pos + 10], buf[*pos + 11]]),
        u16::from_be_bytes([buf[*pos + 12], buf[*pos + 13]]),
        u16::from_be_bytes([buf[*pos + 14], buf[*pos + 15]])
    );

    *pos += isize;

    Ok(ipv6)
}

fn parse_ipv4_addresses(buf: &[u8], pos: &mut usize) -> Result<Vec<Ipv4Addr>> {

    if *pos + 1 > buf.len() {
        return Err(DiscovererError::Deserialize(format!("Distorted UDP packet")));
    }

    let ipv4_addrs_count = usize::from(buf[*pos]);
    *pos += 1;

    let mut ipv4_addrs = Vec::with_capacity(ipv4_addrs_count);

    for _ in 0..ipv4_addrs_count {

        if *pos + 4 > buf.len() {
            return Err(DiscovererError::Deserialize(format!("Distorted UDP packet")));
        }

        let ip = Ipv4Addr::new(buf[*pos], buf[*pos + 1], buf[*pos + 2], buf[*pos + 3]);

        ipv4_addrs.push(ip);
        *pos += 4;
    }

    Ok(ipv4_addrs)

}

fn parse_ipv6_addresses(buf: &[u8], pos: &mut usize) -> Result<Vec<Ipv6Addr>> {

    if *pos + 1 > buf.len() {
        return Err(DiscovererError::Deserialize(format!("Distorted UDP packet")));
    }

    let ipv6_addresses_count = usize::from(buf[*pos]);
    *pos += 1;

    let mut ipv6_addresses = Vec::with_capacity(ipv6_addresses_count);

    for _ in 0..ipv6_addresses_count {

        if *pos + 4 > buf.len() {
            return Err(DiscovererError::Deserialize(format!("Distorted UDP packet")));
        }

        let ipv6 = Ipv6Addr::new(
            u16::from_be_bytes([buf[*pos + 0], buf[*pos + 1]]),
            u16::from_be_bytes([buf[*pos + 2], buf[*pos + 3]]),
            u16::from_be_bytes([buf[*pos + 4], buf[*pos + 5]]),
            u16::from_be_bytes([buf[*pos + 6], buf[*pos + 7]]),
            u16::from_be_bytes([buf[*pos + 8], buf[*pos + 9]]),
            u16::from_be_bytes([buf[*pos + 10], buf[*pos + 11]]),
            u16::from_be_bytes([buf[*pos + 12], buf[*pos + 13]]),
            u16::from_be_bytes([buf[*pos + 14], buf[*pos + 15]])
        );

        ipv6_addresses.push(ipv6);
        *pos += 16;
    }
    
    Ok(ipv6_addresses)    

}
