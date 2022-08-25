use keccak_hash::H256;

use crate::nibbles::Nibbles;

pub type Link = Box<Node>;

#[derive(Debug, Clone, Default)]
pub enum Node {
    #[default]
    Empty,
    Leaf(LeafNode),
    Extension(ExtensionNode),
    Branch(BranchNode),
    Hash(HashNode),
}

impl Node {
    pub fn from_leaf(key: Nibbles, value: Vec<u8>) -> Self {
        let leaf = LeafNode { key, value };
        Node::Leaf(leaf)
    }

    pub fn from_branch(children: [Link; 16], value: Option<Vec<u8>>) -> Self {
        let branch = BranchNode { children, value };
        Node::Branch(branch)
    }

    pub fn from_extension(prefix: Nibbles, node: Node) -> Self {
        let ext = ExtensionNode {
            prefix,
            node: Box::new(node),
        };
        Node::Extension(ext)
    }

    pub fn from_hash(hash: H256) -> Self {
        let hash_node = HashNode { hash };
        Node::Hash(hash_node)
    }
}

#[derive(Debug, Clone)]
pub struct LeafNode {
    pub key: Nibbles,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct BranchNode {
    pub children: [Link; 16],
    pub value: Option<Vec<u8>>,
}

impl BranchNode {
    pub fn insert(&mut self, i: usize, n: Node) {
        if i == 16 {
            match n {
                Node::Leaf(leaf) => {
                    self.value = Some(leaf.value.clone());
                }
                _ => panic!("The n must be leaf node"),
            }
        } else {
            *self.children[i] = n
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ExtensionNode {
    pub prefix: Nibbles,
    pub node: Link,
}

#[derive(Debug, Clone)]
pub struct HashNode {
    pub hash: H256,
}
