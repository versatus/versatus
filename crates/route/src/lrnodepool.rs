
use left_right::{Absorb, ReadHandle, WriteHandle};

// use libp2p::Multiaddr;
use std::net::{Ipv4Addr, IpAddr};

use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet,HashMap},
    result::Result as StdResult,
};

use crate::error::NodePoolError;

pub type Result<T> = StdResult<T, NodePoolError>;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Default)]
pub struct NodeAddr {
    pub node_id: String, // contains PeerId
    pub addr_set: HashSet<IpAddr>,
}

impl NodeAddr {
    pub fn new(
        node_id: String, // contains PeerId
    ) -> NodeAddr {
        NodeAddr {
            node_id,
            addr_set: HashSet::new(),
        }
    }
}

pub type NodeAddrMap = HashMap<String, NodeAddr>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NodePool {
    pub nodes: NodeAddrMap,
}

impl Default for NodePool {
    fn default() -> Self {
        NodePool {
            nodes: HashMap::new()
        }
    }
}

pub enum NodePoolOp {
    Add(String,IpAddr),
    Remove(String,IpAddr),
}

impl Absorb<NodePoolOp> for NodePool {

    fn absorb_first(&mut self, op: &mut NodePoolOp, _: &Self) {

        match op {
            NodePoolOp::Add(key, value) => {

                self.nodes
                    .entry(key.clone())
                    .or_insert(NodeAddr::new(key.clone()))
                    .addr_set.insert(value.clone());
            },
            NodePoolOp::Remove(key, _) => {

                self.nodes.remove_entry(&key.clone());
            },
        }
    }

    fn absorb_second(&mut self, op: NodePoolOp, _: &Self) {

        match op {
            NodePoolOp::Add(key, value) => {

                self.nodes
                    .entry(key.clone())
                    .or_insert(NodeAddr::new(key.clone()))
                    .addr_set.insert(value.clone());
            },
            NodePoolOp::Remove(key, _) => {

                self.nodes.remove_entry(&key.clone());
            },
        }
    }

    fn drop_first(self: Box<Self>) {}

    fn drop_second(self: Box<Self>) {}

    fn sync_with(&mut self, first: &Self) {
        *self = first.clone();
    }

}

pub struct LeftRightNodePoolDB {
    pub read: ReadHandle<NodePool>,
    pub write: WriteHandle<NodePool, NodePoolOp>,
}

impl LeftRightNodePoolDB {

    pub fn new() -> Self {

        let (write, read) = left_right::new::<NodePool, NodePoolOp>();

        LeftRightNodePoolDB { read, write }
    }

    pub fn get(&self) -> Option<NodePool> {
        self.read.enter().map(|guard| guard.clone())
    }

    pub fn add_node(&mut self, node_id: &String, node_addr: IpAddr) -> Result<()> {

        self.write
            .append(NodePoolOp::Add(node_id.clone(), node_addr))
            .publish();

        Ok(())
    }

    pub fn get_node(&mut self, node_id: &String) -> Option<NodeAddr> {

        if node_id.is_empty() {
            return None;
        }

        self.get().and_then(|map| {
            map.nodes
                .get(node_id)
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

    pub fn remove_node(&mut self, node_id: &String) -> Result<()> {

        self.write
            .append(NodePoolOp::Remove(node_id.clone(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))))
            .publish();

        Ok(())
    }

}
