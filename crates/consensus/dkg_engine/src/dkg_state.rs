use std::{
    collections::{BTreeMap, HashMap},
};

use hbbft::{
    crypto::{PublicKey, PublicKeySet, SecretKeyShare},
    sync_key_gen::{Ack, Part, SyncKeyGen},
};
use primitives::NodeId;
use rand::rngs::OsRng;

use crate::{
    prelude::{ReceiverId, SenderId},
};

#[derive(Debug, Default)]
pub struct DkgState {
    part_message_store: HashMap<NodeId, Part>,
    ack_message_store: HashMap<(ReceiverId, SenderId), Ack>,
    peer_public_keys: BTreeMap<NodeId, PublicKey>,
    public_key_set: Option<PublicKeySet>,
    secret_key_share: Option<SecretKeyShare>,
    sync_key_gen: Option<SyncKeyGen<NodeId>>,
    random_number_gen: Option<OsRng>,
}

impl DkgState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.part_message_store.clear();
        self.ack_message_store.clear();
        self.sync_key_gen = None;
        self.random_number_gen = None;
        self.public_key_set = None;
        self.peer_public_keys.clear();
        self.secret_key_share = None;
    }

    pub fn part_message_store_owned(&self) -> HashMap<NodeId, Part> {
        self.part_message_store.clone()
    }

    pub fn part_message_store(&self) -> &HashMap<NodeId, Part> {
        &self.part_message_store
    }

    pub fn part_message_store_mut(&mut self) -> &mut HashMap<NodeId, Part> {
        &mut self.part_message_store
    }

    pub fn set_part_message_store(&mut self, part_message_store: HashMap<NodeId, Part>) {
        self.part_message_store = part_message_store;
    }

    pub fn ack_message_store_owned(&self) -> HashMap<(SenderId, ReceiverId), Ack> {
        self.ack_message_store.clone()
    }

    pub fn ack_message_store(&self) -> &HashMap<(SenderId, ReceiverId), Ack> {
        &self.ack_message_store
    }

    pub fn ack_message_store_mut(&mut self) -> &mut HashMap<(SenderId, ReceiverId), Ack> {
        &mut self.ack_message_store
    }

    pub fn set_ack_message_store(
        &mut self,
        ack_message_store: HashMap<(SenderId, ReceiverId), Ack>,
    ) {
        self.ack_message_store = ack_message_store;
    }

    pub fn peer_public_keys(&self) -> &BTreeMap<NodeId, PublicKey> {
        &self.peer_public_keys
    }

    pub fn peer_public_keys_mut(&mut self) -> &mut BTreeMap<NodeId, PublicKey> {
        &mut self.peer_public_keys
    }

    pub fn peer_public_keys_owned(&self) -> BTreeMap<NodeId, PublicKey> {
        self.peer_public_keys.clone()
    }

    pub fn set_peer_public_keys(&mut self, peer_public_keys: BTreeMap<NodeId, PublicKey>) {
        self.peer_public_keys = peer_public_keys;
    }

    pub fn public_key_set(&self) -> &Option<PublicKeySet> {
        &self.public_key_set
    }

    pub fn public_key_set_mut(&mut self) -> &mut Option<PublicKeySet> {
        &mut self.public_key_set
    }

    pub fn public_key_set_owned(&self) -> Option<PublicKeySet> {
        self.public_key_set.clone()
    }

    pub fn set_public_key_set(&mut self, public_key_set: Option<PublicKeySet>) {
        self.public_key_set = public_key_set;
    }

    pub fn secret_key_share(&self) -> &Option<SecretKeyShare> {
        &self.secret_key_share
    }

    pub fn secret_key_share_mut(&mut self) -> &mut Option<SecretKeyShare> {
        &mut self.secret_key_share
    }

    pub fn secret_key_share_owned(&self) -> Option<SecretKeyShare> {
        self.secret_key_share.clone()
    }

    pub fn set_secret_key_share(&mut self, secret_key_share: Option<SecretKeyShare>) {
        self.secret_key_share = secret_key_share;
    }

    pub fn sync_key_gen(&self) -> &Option<SyncKeyGen<NodeId>> {
        &self.sync_key_gen
    }

    pub fn sync_key_gen_mut(&mut self) -> &mut Option<SyncKeyGen<NodeId>> {
        &mut self.sync_key_gen
    }

    pub fn set_sync_key_gen(&mut self, sync_key_gen: Option<SyncKeyGen<NodeId>>) {
        self.sync_key_gen = sync_key_gen;
    }

    pub fn random_number_gen_owned(&self) -> Option<OsRng> {
        self.random_number_gen.clone()
    }

    pub fn random_number_gen(&self) -> &Option<OsRng> {
        &self.random_number_gen
    }

    pub fn random_number_gen_mut(&mut self) -> &mut Option<OsRng> {
        &mut self.random_number_gen
    }

    pub fn set_random_number_gen(&mut self, random_number_gen: Option<OsRng>) {
        self.random_number_gen = random_number_gen;
    }

    pub fn add_peer_public_key(&mut self, node_id: NodeId, public_key: PublicKey) {
        self.peer_public_keys.insert(node_id, public_key);
    }
}
