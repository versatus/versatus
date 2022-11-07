
use chrono::Utc;
use left_right::{Absorb, ReadHandle, WriteHandle};

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    cmp::{Ordering, min}, 
    borrow::Cow,
    hash::{Hash, Hasher},
};

use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet,HashMap},
    result::Result as StdResult,
};

use crate::{error::NodePoolError, message::NodeType, MAX_CONNECTED_NODES};

pub type Result<T> = StdResult<T, NodePoolError>;

#[derive(Eq, Serialize, Deserialize, Debug, Default)]
pub struct NodeKey<'m> {
    pub node_id: Cow<'m, str>,  // contains PeerId
    pub node_distance: i64,     // network distance in millis
    pub node_type: u8,          // 
    pub node_timestamp: i64     // last update
}

impl<'m> NodeKey<'m> {
    pub fn new<M: Into<Cow<'m, str>>>(
            node_id: M,
            node_distance: i64,
            node_type: NodeType) -> Self {
        Self {
            node_id: node_id.into(),
            node_distance,
            node_type: node_type as u8,
            node_timestamp: Utc::now().timestamp_millis(),
        }
    }
}

impl PartialOrd for NodeKey<'_> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.node_distance.partial_cmp(&other.node_distance)
    }
}

impl Clone for NodeKey<'_> {
    fn clone(&self) -> Self {
        Self { 
            node_id: self.node_id.clone(),
            node_distance: self.node_distance.clone(),
            node_type: self.node_type.clone(),
            node_timestamp: Utc::now().timestamp_millis()
        }
    }
}

impl PartialEq for NodeKey<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
        && self.node_distance == other.node_distance
    }
    #[inline]
    fn ne(&self, other: &Self) -> bool {
        !(*self).eq(other)
    }
}

impl Hash for NodeKey<'_> {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.node_id.hash(hasher);
    }
}

impl Ord for NodeKey<'_> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.node_distance.cmp(&other.node_distance)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Default)]
pub struct NodeAddr<'m> {
    pub node_key: NodeKey<'m>,
    pub addr_set: HashSet<IpAddr>,
}

impl<'m> NodeAddr<'m> {
    pub fn new<N: Into<Cow<'m, str>>>(
        node_id: N,             // contains PeerId
        node_distance: i64,     // network distance in millis
        node_type: NodeType     // Type
    ) -> NodeAddr<'m> {
        NodeAddr {
            node_key: NodeKey::new(node_id, node_distance, node_type),
            addr_set: HashSet::new(),
        }
    }
}

pub type NodeAddrMap<'m> = HashMap<NodeKey<'m>, NodeAddr<'m>>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NodePool<'m> {
    pub nodes: NodeAddrMap<'m>,
}

impl<'m> Default for NodePool<'m> {
    fn default() -> Self {
        NodePool {
            nodes: HashMap::new()
        }
    }
}

pub enum NodePoolOp {
    Add(String,i64,NodeType,IpAddr),
    AddAddr(String,IpAddr),
    UpdateDistance(String,i64),
    RemoveInactiveNodes(i64),
    Remove(String),
}

impl<'m> Absorb<NodePoolOp> for NodePool<'m> {

    fn absorb_first(&mut self, op: &mut NodePoolOp, _: &Self) {

        match op {
            NodePoolOp::Add(id, distance, mtype, value) => {

                let key = NodeKey::new(id.clone(), *distance, *mtype);

                if let Some(entry) = self.nodes.get_key_value(&key) {
                    let mut new_key = entry.0.clone();
                    new_key.node_distance = *distance;
                    if let Some(mut node_addr) = self.nodes.remove(&key) {
                        node_addr.addr_set.insert(value.clone());
                        self.nodes.insert(new_key, node_addr);
                    }
                } else {
                    let mut node_addr = NodeAddr::<'m>::new(id.clone(), *distance, *mtype);
                    node_addr.addr_set.insert(value.clone());
                    self.nodes
                        .insert(key, node_addr);
                }
            },
            NodePoolOp::AddAddr(id, value) => {

                let key = NodeKey::new(id.clone(), 0, NodeType::ALL);

                if let Some(entry) = self.nodes.get_key_value(&key) {
                    let new_key = entry.0.clone();
                    if let Some(mut node_addr) = self.nodes.remove(&key) {
                        node_addr.addr_set.insert(value.clone());
                        self.nodes.insert(new_key, node_addr);
                    }
                }
            },
            NodePoolOp::UpdateDistance(id, value) => {

                let key = NodeKey::new(id.clone(), *value, NodeType::ALL);
                
                if let Some(entry) = self.nodes.get_key_value(&key) {
                    let mut new_key = entry.0.clone();
                    new_key.node_distance = *value;
                    if let Some(node_addr) = self.nodes.remove(&key) {
                        self.nodes.insert(new_key, node_addr);
                    }
                }
            },
            NodePoolOp::RemoveInactiveNodes(max_inactivity) => {

                let now = Utc::now().timestamp_millis();
                let max_inactivity_in_millis = *max_inactivity * 1000;

                self.nodes
                    .retain(|k, _| (now - k.node_timestamp) < max_inactivity_in_millis);                    
            },
            NodePoolOp::Remove(id) => {

                let key = NodeKey::new(id.clone(), 0, NodeType::Validator);

                self.nodes.remove_entry(&key.clone());
            },
        }
    }

    fn sync_with(&mut self, first: &Self) {
        *self = first.clone();
    }

}

pub struct LeftRightNodePoolDB<'m> {
    pub read: ReadHandle<NodePool<'m>>,
    pub write: WriteHandle<NodePool<'m>, NodePoolOp>,
}

unsafe impl Sync for LeftRightNodePoolDB<'_> {}

impl<'m> LeftRightNodePoolDB<'m> {

    pub fn new() -> Self {

        let (write, read) = left_right::new::<NodePool, NodePoolOp>();

        LeftRightNodePoolDB { read, write }
    }

    pub fn get(&self) -> Option<NodePool> {
        self.read.enter().map(|guard| guard.clone())
    }

    pub fn add_node(&mut self, node_id: String, node_distance: i64, node_type: NodeType, node_addr: IpAddr) -> &mut Self {

        self.write
            .append(NodePoolOp::Add(node_id, node_distance, node_type, node_addr))
            .publish();

        self
    }

    pub fn add_addr(&mut self, node_id: String, node_addr: IpAddr) -> &mut Self {

        self.write
            .append(NodePoolOp::AddAddr(node_id, node_addr))
            .publish();

        self
    }

    pub fn add_addrs(&mut self, node_id: String, node_addrs: &Vec<IpAddr>) -> &mut Self {

        for ip in node_addrs {

            self.write
                .append(NodePoolOp::AddAddr(node_id.clone(), ip.clone()));
        }
        
        self.write.publish();

        self
    }

    pub fn add_addrs_v4(&mut self, node_id: String, node_addrs: &Vec<Ipv4Addr>) -> &mut Self {

        for ip in node_addrs {

            self.write
                .append(NodePoolOp::AddAddr(node_id.clone(), IpAddr::V4(ip.clone())));
        }
        
        self.write.publish();

        self
    }

    pub fn add_addrs_v6(&mut self, node_id: String, node_addrs: &Vec<Ipv6Addr>) -> &mut Self {

        for ip in node_addrs {

            self.write
                .append(NodePoolOp::AddAddr(node_id.clone(), IpAddr::V6(ip.clone())));
        }
        
        self.write.publish();

        self
    }

    pub fn update_node_distance(&mut self, node_id: String, node_distance: i64) -> &mut Self {

        self.write.append(NodePoolOp::UpdateDistance(node_id.clone(), node_distance));

        self.write.publish();

        self
    }

    pub fn remove_inactive_nodes(&mut self, max_inactivity_duration_in_secs: i64) -> &mut Self  {

        self.write.append(NodePoolOp::RemoveInactiveNodes(max_inactivity_duration_in_secs));

        self.write.publish();

        self
    }

    pub fn get_node(&mut self, node_id: &'m String) -> Option<NodeAddr> {

        if node_id.is_empty() {
            return None;
        }

        let key = NodeKey::new(node_id, 0, NodeType::Validator);

        self.get().and_then(|map| {
            map.nodes
                .get(&key)
                .and_then(|n| Some(n.clone()))
        })
    }

    pub fn get_all_nodes(&mut self) -> Option<Vec<NodeAddr>> {

        self.get().and_then(|map| {

            let vec: Vec<NodeAddr> = map
                                    .nodes
                                    .values()
                                    .cloned()
                                    .collect();
            Some(vec)
        })
    }

    pub fn get_cluster(&mut self, node_type: NodeType) -> Option<Vec<NodeAddr>> {

        self.get().and_then(|map| {

            let vec: Vec<NodeAddr> = match node_type {
                NodeType::ALL => {
                    map
                    .nodes
                    .values()
                    .cloned()
                    .collect()
                }
                _ =>  {
                    map
                    .nodes
                    .values()
                    .filter(|n| n.node_key.node_type == node_type as u8)
                    .cloned()
                    .collect()
                }
            };

            let cluster_size = min(vec.len(), MAX_CONNECTED_NODES);

            Some(vec[0..cluster_size].to_vec())
        })
    }

    pub fn remove_node(&mut self, node_id: &String) -> Result<()> {

        self.write
            .append(NodePoolOp::Remove(node_id.clone()))
            .publish();

        Ok(())
    }

}
