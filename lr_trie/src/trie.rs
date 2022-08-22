use nibble_vec::NibbleVec;
use nibbler::{nibble::Nibble, nibbles::Nibbles};
use std::collections::HashMap;

// TODO: make branches configurable
const BRANCHING_FACTOR: usize = 255;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// trait Tree<K: AsRef<[u8]>> {
//     fn get(&self, key: K) -> Result<Node>;
//     fn insert(&mut self, key: K, value: Node) -> Result<Node>;
//     fn extend(&mut self);
//     fn remove(&mut self);
// }

trait MerkleTree {
    fn root(&self) -> String;
}

// #[derive(Debug, Clone)]
// pub struct Leaf {
//     pub nibble: Nibble,
//     pub value: usize,
// }
//
// #[derive(Clone, Debug, Default)]
// pub struct Branch {
//     pub keys: [Option<usize>; 16],
//     pub value: Option<usize>,
// }
//
// #[derive(Debug, Clone)]
// pub struct Extension {
//     pub nibble: Nibble,
//     pub key: usize,
// }
//
// #[derive(Debug, Clone)]
// enum Node {
//     Empty,
//     Leaf(Leaf),
//     Branch(Box<Branch>),
//     Extension(Extension),
// }
//
// #[derive(Debug)]
// enum Action {
//     Root,
//     BranchKey(u8, Leaf),
//     Extension(Extension, u32),
//     Leaf(Leaf, u32),
// }
//

type Link<K, V> = Box<Node<K, V>>;

/// Type of node in the trie and essential information thereof.
#[derive(Eq, PartialEq, Clone, Debug)]
// pub enum Node<'a> {
pub enum Node<K, V>
where
    K: AsRef<[u8]>,
    V: AsRef<[u8]>,
{
    /// Null trie node; could be an empty root or an empty branch entry.
    Empty,
    /// Leaf node; has key slice and value. Value may not be empty.
    // Leaf(NibbleSlice<'a>, Value<'a>),
    Leaf(K, V),
    /// Extension node; has key slice and node data. Data may not be null.
    // Extension(NibbleSlice<'a>, NodeHandle<'a>),
    Extension(K, Link<K, V>),
    /// Branch node; has slice of child nodes (each possibly null)
    /// and an optional immediate node data.
    // Branch(
    //     [Option<NodeHandle<'a>>; nibble_ops::NIBBLE_LENGTH],
    //     Option<Value<'a>>,
    // ),
    // //
    Branch(V, [Link<K, V>; BRANCHING_FACTOR]),
}

//
// /// Branch node with support for a nibble (when extension nodes are not used).
// NibbledBranch(
//     NibbleSlice<'a>,
//     [Option<NodeHandle<'a>>; nibble_ops::NIBBLE_LENGTH],
//     Option<Value<'a>>,
// ),

#[derive(Debug, Default)]
struct Trie<K: AsRef<[u8]>> {
    root_index: usize,
    db: HashMap<usize, Node<K, K>>,
}

impl<K: AsRef<[u8]>> Trie<K> {
    pub fn new() -> Self {
        Trie {
            root_index: 0,
            db: Default::default(),
        }
    }
}

// impl<K: AsRef<[u8]>> MerkleTree for Trie<K> {
impl<K: AsRef<[u8]>> Trie<K> {
    // pub fn root(&self) -> String {
    //     // TODO: impl hasher
    //     String::from("")
    //     // self.root.hash()
    // }

    fn root(&self) -> String {
        String::from("")
    }
}

// impl<K: AsRef<[u8]>> Tree<K> for Trie<K> {
impl<K: AsRef<[u8]>> Trie<K> {
    // fn get(&self, key: &Nibble) -> Result<T> {
    fn get(&self, key: K) -> Result<Node<K, K>> {
        Ok(Node::Empty)
    }

    //
    // Insert adds a key value pair to the trie
    // In general, the rule is:
    // - When stopped at an EmptyNode, replace it with a new LeafNode with the remaining path.
    // - When stopped at a LeafNode, convert it to an ExtensionNode and add a new branch and a new LeafNode.
    // - When stopped at an ExtensionNode, convert it to another ExtensionNode with shorter path and create a new BranchNode points to the ExtensionNode.
    fn insert(&mut self, key: K, value: K) -> Result<Node<K, K>> {
        // TODO: create a nibble to use as path

        let nibbles = key;
        let mut key = self.root_index;
        // let mut path = leaf.nibble;

        let node = self.db.get_mut(&self.root_index);
        loop {
            if let Some(node) = node {
                match node {
                    &mut Node::Empty => {
                        // if EmptyNode, replace it with a new LeafNode with the remaining path.

                        *node = Node::Leaf(nibbles, value);

                        break Ok(Node::Empty);
                    }
                    Node::Leaf(_, _) => {
                        //
                        break Ok(Node::Empty);
                    }
                    Node::Branch(_, _) => {
                        //
                        break Ok(Node::Empty);
                    }
                    Node::Extension(_, _) => {
                        //
                        break Ok(Node::Empty);
                    }
                };
            } else {
                // should not happen
                panic!("map entry for root node not found");
            }
        }
    }

    /*

        // fn insert(&mut self, key: &Nibble, value: T) -> Result<T> {
        fn insert(&mut self, key: K, value: Node) -> Result<Node> {
            // let value = self.arena.push(&arena[leaf.value]);

            let key = key.as_ref();
            let value = value.as_ref();
            let data = &[key, value];

            // let arena = &ArenaSlice(data.as_ref());

            // let nibble = Nibble {
            //     data: 0,
            //     start: 0,
            //     end: key.len() as u32 * 2,
            // };

            let nibble = Nibble::new();

            let leaf = Leaf { nibble, value: 1 };

            let mut key = self.root_index;
            let mut path = leaf.nibble;

            let action = loop {
                // match self.db.get_mut(&mut key) {
                match self.db.get_mut(&mut key) {
                    //
                    //
                    //
                    // BEGIN BRANCH NODE
                    Some(Node::Branch(ref mut branch)) => {
                        if let Some((u, n)) = path.pop_front(arena) {
                            let mut k = branch.keys[u as usize];
                            match k {
                                Some(ref k) => {
                                    key = *k;
                                    path = n;
                                }
                                None => {
                                    // update branch key
                                    let nibble = n.copy(arena, &mut self.arena);
                                    break Action::BranchKey(u, Leaf { nibble, value });
                                }
                            }
                        } else {
                            // update branch value
                            let old_value = mem::replace(&mut branch.value, Some(value));
                            let arena = &self.arena;
                            return old_value.map(move |v| &arena[v]);
                        }
                    }
                    // END BRANCH NODE
                    //
                    //
                    //
                    //
                    // BEGIN EXTENSION NODE
                    Some(Node::Extension(ref extension)) => {
                        let (left, right) = path.split_at(extension.nibble.len());
                        let pos = extension
                            .nibble
                            .iter(&self.arena)
                            .zip(left.iter(arena))
                            .position(|(u, v)| u != v);

                        if let Some(p) = pos {
                            dbg!("extension doesn't start with path nor path starts with extension");
                            break Action::Extension(extension.clone(), p as u32);
                        } else {
                            dbg!(
                                "path {} starts with extension {}",
                                path.len(),
                                extension.nibble.len()
                            );
                            path = right.unwrap_or_default();
                            key = extension.key;
                        }
                    }
                    // END EXTENSION NODE
                    //
                    //
                    //
                    //
                    // BEGIN LEAF NODE
                    Some(Node::Leaf(ref mut leaf)) => {
                        let (left, right) = path.split_at(leaf.nibble.len());
                        let pos = leaf
                            .nibble
                            .iter(&self.arena)
                            .zip(left.iter(arena))
                            .position(|(u, v)| u != v);
                        if let Some(p) = pos {
                            dbg!("leaf doesn't start with path nor path starts with leaf");
                            break Action::Leaf(leaf.clone(), p as u32);
                        } else if let Some(_right) = right {
                            dbg!("path starts with leaf (right: {:?})", _right);
                            break Action::Leaf(leaf.clone(), leaf.nibble.len());
                        } else if path.len() == leaf.nibble.len() {
                            dbg!("nibble == leaf => replace leaf");
                            let old_val = mem::replace(&mut leaf.value, value);
                            return Some(&self.arena[old_val]);
                        } else {
                            dbg!("leaf starts with path");
                            break Action::Leaf(leaf.clone(), path.len());
                        }
                    }
                    // END LEAF NODE
                    //
                    //
                    //
                    _ => break Action::Root,
                }
            };

            self.execute_action(action, key, value, &path, arena);

            Node::Empty
        }

        fn insert_leaf_node() {}
        fn insert_branch_node() {}
        fn insert_extension_node() {}

        #[inline(always)]
        // fn execute_action<A>(
        fn execute_action(
            &mut self,
            action: Action,
            // mut key: Index,
            mut key: usize,
            value: usize,
            path: &Nibble,
            // arena: &A,
        ) -> Option<&[u8]>
    // where
            // A: ::std::ops::Index<usize, Output = [u8]>,
        {
            dbg!(" -- Inserting {:?}", action);

            match action {
                Action::BranchKey(u, new_leaf) => {
                    let new_key = self.db.push_node(Node::Leaf(new_leaf));
                    if let Node::Branch(ref mut branch) = self.db.get_mut(&mut key)? {
                        branch.keys[u as usize] = Some(new_key);
                    }
                }
                Action::Extension(ext, offset) => {
                    let old = self.db.remove(&key);

                    let (_, path) = path.split_at(offset);
                    let (ext_left, ext_right) = ext.nibble.split_at(offset);

                    let mut branch = Branch::default();

                    if let Some((u, path)) = path.and_then(|p| p.pop_front(arena)) {
                        // let nibble = path.copy(arena, &mut self.arena);

                        let new_key = self.db.push_node(Node::Leaf(Leaf { nibble, value }));

                        branch.keys[u as usize] = Some(new_key);
                    } else {
                        branch.value = Some(value);
                    }

                    if let Some((u, nibble)) = ext_right.and_then(|n| n.pop_front(&self.arena)) {
                        let new_key = if nibble.len() == 0 {
                            // there is no nibble extension so the extension is useless
                            // and we can directly refer to the nibble key
                            ext.key
                        } else {
                            let ext = Extension {
                                nibble,
                                key: ext.key,
                            };
                            self.db.push_node(Node::Extension(ext))
                        };
                        branch.keys[u as usize] = Some(new_key);
                    } else {
                        panic!("extension nibble too short");
                    }

                    if offset > 0 {
                        let branch_key = self.db.push_node(Node::Branch(Box::new(branch)));
                        let ext = Extension {
                            nibble: ext_left,
                            key: branch_key,
                        };
                        self.db.insert_node(key, Node::Extension(ext));
                    } else {
                        self.db.insert_node(key, Node::Branch(Box::new(branch)));
                    }
                }
                Action::Leaf(leaf, offset) => {
                    self.db.remove(&key);
                    let mut branch = Branch::default();
                    dbg!("leaf: {:?}, path: {:?}, offset: {}", leaf, path, offset);
                    let (_, path) = path.split_at(offset);
                    if let Some((u, path)) = path.and_then(|p| p.pop_front(arena)) {
                        dbg!("new leaf: {:?}", path);
                        let nibble = path.copy(arena, &mut self.arena);
                        let new_key = self.db.push_node(Node::Leaf(Leaf { nibble, value }));
                        branch.keys[u as usize] = Some(new_key);
                    } else {
                        dbg!("new leaf as branch value");
                        branch.value = Some(value);
                    }
                    let (leaf_left, leaf_right) = leaf.nibble.split_at(offset);
                    if let Some((u, nibble)) = leaf_right.and_then(|n| n.pop_front(&self.arena)) {
                        dbg!("existing leaf: {:?}", nibble);
                        let leaf = Leaf {
                            nibble,
                            value: leaf.value,
                        };
                        let new_key = self.db.push_node(Node::Leaf(leaf));
                        branch.keys[u as usize] = Some(new_key);
                    } else {
                        dbg!("existing leaf as branch value");
                        branch.value = Some(leaf.value);
                    }
                    if offset > 0 {
                        let branch_key = self.db.push_node(Node::Branch(Box::new(branch)));
                        let ext = Extension {
                            nibble: leaf_left,
                            key: branch_key,
                        };
                        self.db.insert_node(key, Node::Extension(ext));
                    } else {
                        self.db.insert_node(key, Node::Branch(Box::new(branch)));
                    }
                }
                Action::Root => {
                    let nibble = path.copy(arena, &mut self.arena);
                    self.db.insert_node(key, Node::Leaf(Leaf { nibble, value }));
                }
            }
            None
        }
        */

    fn extend(&mut self) {
        todo!()
    }

    fn remove(&mut self) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_get_value() {
        let mut trie: Trie<TrieNode> = Trie::new();

        // let key = Nibble::new(true, false, true, false);
        let key = &[1, 2, 3, 4];

        let data = trie.get(&key).unwrap();

        trie.insert(&key, data.clone());

        dbg!(data);
    }
}

// impl Default for TrieNode {
//     fn default() -> Self {
//         Self {
//             key: Nibble::new(false, false, false, false),
//             node_type: Default::default(),
//             value: Default::default(),
//             branches: Default::default(),
//         }
//     }
// }
//
// impl TrieNode {
//     pub fn new() -> Self {
//         Default::default()
//     }
// }
//
// impl Node for TrieNode {}
//
//
// type Link = Box<TrieNode>;

// #[derive(Debug, Clone)]
// struct TrieNode {
//     key: Nibble,
//     node_type: NodeType,
//     //value: T
//     value: usize,
//     // branches: [Link; BRANCHING_FACTOR],
//     branches: Vec<Link>,
// }
