use std::{
    borrow::Cow,
    cmp::{min, Ordering},
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    result::Result as StdResult,
};

use chrono::Utc;
use left_right::{Absorb, ReadHandle, WriteHandle};
use primitives::NodeType;
use serde::{Deserialize, Serialize};

use crate::{error::NodePoolError, MAX_CONNECTED_NODES};

// TODO, fix the compiler.
#[allow(dead_code)]
pub type Result<T> = StdResult<T, NodePoolError>;

/// A key containing state of a single node.
#[derive(Eq, Serialize, Deserialize, Debug, Default)]
pub struct NodeKey<'m> {
    pub node_id: Cow<'m, str>, // contains PeerId.
    pub node_distance: i64,    /* network distance in millis, used for sorting by the closest
                                * and fasters neighbours. */
    pub node_type: u8,       // information about node's type.
    pub node_timestamp: i64, // last contact with the node, allows to distinguish inactive nodes.
}

impl<'m> NodeKey<'m> {
    /// Builds a new key
    ///
    /// # Arguments
    /// * `node_id`             - NodeId
    /// * `node_distance`       - Node distance in milliseconds
    /// * `node_type`           - NodeType - Archive, ... etc
    pub fn new<M: Into<Cow<'m, str>>>(node_id: M, node_distance: i64, node_type: NodeType) -> Self {
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
            node_distance: self.node_distance,
            node_type: self.node_type,
            node_timestamp: Utc::now().timestamp_millis(),
        }
    }
}

impl PartialEq for NodeKey<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id && self.node_distance == other.node_distance
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

/// Node details and addresses, discovered during process of syncing.
impl<'m> NodeAddr<'m> {
    /// Builds a NodeAddr
    ///
    /// # Arguments
    /// * `node_id`             - NodeId
    /// * `node_distance`       - Node distance in milliseconds
    /// * `node_type`           - NodeType - Archive, ... etc
    pub fn new<N: Into<Cow<'m, str>>>(
        node_id: N,          // contains PeerId
        node_distance: i64,  // network distance in millis
        node_type: NodeType, // Type
    ) -> NodeAddr<'m> {
        NodeAddr {
            node_key: NodeKey::new(node_id, node_distance, node_type),
            addr_set: HashSet::new(),
        }
    }
}

pub type NodeAddrMap<'m> = HashMap<NodeKey<'m>, NodeAddr<'m>>;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct NodePool<'m> {
    pub nodes: NodeAddrMap<'m>,
}

/// All the map operations.
// TODO, fix the compiler
#[allow(dead_code)]
pub enum NodePoolOp {
    Add(String, i64, NodeType, IpAddr), // add a new node.
    AddAddr(String, IpAddr),            // add an address to already exisiting node.
    UpdateDistance(String, i64),        // update current network distance. TODO
    RemoveInactiveNodes(i64),           /* remove inactive node by max distance from the map
                                         * and network. */
    Remove(String), // remove inactive node by ID. TODO
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
                        node_addr.addr_set.insert(*value);
                        self.nodes.insert(new_key, node_addr);
                    }
                } else {
                    let mut node_addr = NodeAddr::<'m>::new(id.clone(), *distance, *mtype);
                    node_addr.addr_set.insert(*value);
                    self.nodes.insert(key, node_addr);
                }
            },
            NodePoolOp::AddAddr(id, value) => {
                let key = NodeKey::new(id.clone(), 0, NodeType::Full);

                if let Some(entry) = self.nodes.get_key_value(&key) {
                    let new_key = entry.0.clone();
                    if let Some(mut node_addr) = self.nodes.remove(&key) {
                        node_addr.addr_set.insert(*value);
                        self.nodes.insert(new_key, node_addr);
                    }
                }
            },
            NodePoolOp::UpdateDistance(id, value) => {
                let key = NodeKey::new(id.clone(), *value, NodeType::Full);

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
    /// Builds a LeftRightNodePoolDB
    pub fn new() -> Self {
        let (write, read) = left_right::new::<NodePool, NodePoolOp>();

        LeftRightNodePoolDB { read, write }
    }

    pub fn get(&self) -> Option<NodePool> {
        self.read.enter().map(|guard| guard.clone())
    }

    /// Add new node
    ///
    /// # Arguments
    /// * `node_id`             - NodeId
    /// * `node_distance`       - Node distance in milliseconds
    /// * `node_type`           - NodeType - Archive, ... etc
    /// * `node_addr`           - received address for the node.
    pub fn add_node(
        &mut self,
        node_id: String,
        node_distance: i64,
        node_type: NodeType,
        node_addr: IpAddr,
    ) -> &mut Self {
        self.write
            .append(NodePoolOp::Add(
                node_id,
                node_distance,
                node_type,
                node_addr,
            ))
            .publish();

        self
    }

    /// Add address to an existing Node.
    ///
    /// # Arguments
    /// * `node_id`             - NodeId
    /// * `node_addr`           - received address for the node.
    pub fn _add_addr(&mut self, node_id: String, node_addr: IpAddr) -> &mut Self {
        self.write
            .append(NodePoolOp::AddAddr(node_id, node_addr))
            .publish();

        self
    }

    /// Add addresses to an existing Node.
    ///
    /// # Arguments
    /// * `node_id`             - NodeId
    /// * `node_addrs`          - Vec received address for the node.
    pub fn _add_addrs(&mut self, node_id: String, node_addrs: &Vec<IpAddr>) -> &mut Self {
        for ip in node_addrs {
            self.write.append(NodePoolOp::AddAddr(node_id.clone(), *ip));
        }

        self.write.publish();

        self
    }

    /// Add addresses IPv4 to an existing Node.
    ///
    /// # Arguments
    /// * `node_id`             - NodeId
    /// * `node_addrs`          - Vec received address IPv4 for the node.
    pub fn add_addrs_v4(&mut self, node_id: String, node_addrs: &Vec<Ipv4Addr>) -> &mut Self {
        for ip in node_addrs {
            self.write
                .append(NodePoolOp::AddAddr(node_id.clone(), IpAddr::V4(*ip)));
        }

        self.write.publish();

        self
    }

    ///
    /// Add addresses IPv6 to an existing Node.
    ///
    /// # Arguments
    /// * `node_id`             - NodeId
    /// * `node_addrs`          - Vec received address IPv6 for the node.
    pub fn add_addrs_v6(&mut self, node_id: String, node_addrs: &Vec<Ipv6Addr>) -> &mut Self {
        for ip in node_addrs {
            self.write
                .append(NodePoolOp::AddAddr(node_id.clone(), IpAddr::V6(*ip)));
        }

        self.write.publish();

        self
    }

    ///
    /// Update an existing Node distance.
    ///
    /// # Arguments
    ///
    /// * `node_id`             - NodeId
    /// * `node_distance`       - Node distance in milliseconds
    pub fn _update_node_distance(&mut self, node_id: String, node_distance: i64) -> &mut Self {
        self.write
            .append(NodePoolOp::UpdateDistance(node_id, node_distance));

        self.write.publish();

        self
    }

    ///
    /// Remove inactive nodes
    ///
    /// # Arguments
    ///
    /// * `max_inactivity_duration_in_secs`             - maximum inactivity
    ///   period for a node
    pub fn remove_inactive_nodes(&mut self, max_inactivity_duration_in_secs: i64) -> &mut Self {
        self.write.append(NodePoolOp::RemoveInactiveNodes(
            max_inactivity_duration_in_secs,
        ));

        self.write.publish();

        self
    }

    /// Retrieve an existing Node.
    ///
    /// # Arguments
    ///
    /// * `node_id`             - NodeId
    pub fn _get_node(&mut self, node_id: &'m String) -> Option<NodeAddr> {
        if node_id.is_empty() {
            return None;
        }

        let key = NodeKey::new(node_id, 0, NodeType::Validator);

        self.get().and_then(|map| map.nodes.get(&key).cloned())
    }

    /// Retrieve all the existing registered Nodes.
    #[allow(dead_code)]
    pub fn get_all_nodes(&mut self) -> Option<Vec<NodeAddr>> {
        self.get()
            .map(|pool| pool.nodes.values().cloned().collect())
    }

    ///
    /// Retrieve all the existing registered Nodes from the current sub cluster
    /// of Nodes.
    ///
    /// # Arguments
    /// * `node_type`           - NodeType - Archive, ... etc
    pub fn get_cluster(&mut self, node_type: NodeType) -> Option<Vec<NodeAddr>> {
        self.get().map(|pool| {
            let vec: Vec<NodeAddr> = match node_type {
                NodeType::Full => pool.nodes.values().cloned().collect(),
                _ => pool
                    .nodes
                    .values()
                    .filter(|n| n.node_key.node_type == node_type as u8)
                    .cloned()
                    .collect(),
            };

            let cluster_size = min(vec.len(), MAX_CONNECTED_NODES);

            vec[0..cluster_size].to_vec()
        })
    }

    /// Remove an existing Node.
    ///
    /// # Arguments
    ///
    /// * `node_id`             - NodeId
    pub fn _remove_node(&mut self, node_id: &str) -> Result<()> {
        self.write
            .append(NodePoolOp::Remove(node_id.to_owned()))
            .publish();

        Ok(())
    }
}
