use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    net::SocketAddr,
};

use primitives::{NodeType, PeerId, TxHash, TxHashString};
use serde::{Deserialize, Serialize};
use telemetry::{error, info};
use tokio::sync::{
    broadcast::{self, Sender},
    mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::{txn::Txn, Error, Result};

pub type Subscriber = UnboundedSender<Event>;
pub type Publisher = UnboundedSender<(Topic, Event)>;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PeerData {
    pub address: SocketAddr,
    pub node_type: NodeType,
    pub peer_id: PeerId,
}

// NOTE: naming convention for events goes as follows:
// <Subject><Verb, in past tense>, e.g. ObjectCreated
// TODO: Replace Vec<u8>'s with proper data structs in enum wariants
// once definitions of those are moved into primitives.
#[derive(Default, Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Event {
    #[default]
    NoOp,
    Stop,
    /// New txn came from network, requires validation
    #[deprecated(note = "replaced by NewTxnCreated")]
    TxnCreated(Vec<u8>),
    /// New txn came from network, requires validation
    NewTxnCreated(Txn),
    /// Single txn validated
    TxnValidated(Vec<u8>),
    /// Batch of validated txns
    TxnBatchValidated(Vec<u8>),
    TxnAddedToMempool(TxHashString),
    BlockConfirmed(Vec<u8>),
    ClaimCreated(Vec<u8>),
    ClaimProcessed(Vec<u8>),
    UpdateLastBlock(Vec<u8>),
    ClaimAbandoned(String, Vec<u8>),
    SlashClaims(Vec<String>),
    CheckAbandoned,
    PeerRequestedStateSync(PeerData),

    /// A peer joined the network, should be added to the node's peer list
    PeerJoined(PeerData),

    /// Peer abandoned the network. Should be removed from the node's peer list
    PeerLeft(SocketAddr),
    // SendTxn(u32, String, u128), // address number, receiver address, amount
    // ProcessTxnValidator(Vec<u8>),
    // PendingBlock(Vec<u8>, String),
    // InvalidBlock(Vec<u8>),
    // ProcessClaim(Vec<u8>),
    // CheckStateUpdateStatus((u128, Vec<u8>, u128)),
    // StateUpdateCompleted(Vec<u8>),
    // StoreStateDbChunk(Vec<u8>, Vec<u8>, u32, u32),
    // SendState(String, u128),
    // SendMessage(SocketAddr, Message),
    // GetBalance(u32),
    // SendGenesis(String),
    // SendStateComponents(String, Vec<u8>, String),
    // GetStateComponents(String, Vec<u8>, String),
    // RequestedComponents(String, Vec<u8>, String, String),
    // StoreStateComponents(Vec<u8>, ComponentTypes),
    // StoreChild(Vec<u8>),
    // StoreParent(Vec<u8>),
    // StoreGenesis(Vec<u8>),
    // StoreLedger(Vec<u8>),
    // StoreNetworkState(Vec<u8>),
    // StateUpdateComponents(Vec<u8>, ComponentTypes),
    // UpdateAppMiner(Vec<u8>),
    // UpdateAppBlockchain(Vec<u8>),
    // UpdateAppMessageCache(Vec<u8>),
    // UpdateAppWallet(Vec<u8>),
    // Publish(Vec<u8>),
    // Gossip(Vec<u8>),
    // AddNewPeer(String, String),
    // AddKnownPeers(Vec<u8>),
    // AddExplicitPeer(String, String),
    // ProcessPacket((Packet, SocketAddr)),
    // Bootstrap(String, String),
    // SendPing(String),
    // ReturnPong(Vec<u8>, String),
    // InitHandshake(String),
    // ReciprocateHandshake(String, String, String),
    // CompleteHandshake(String, String, String),
    // ProcessAck(String, u32, String),
    // CleanInbox(String),
    // StartMiner,
    // GetHeight,
    // MineBlock,
    // MineGenesis,
    // StopMine,
    // GetState,
    // ProcessBacklog,
    // SendAddress,
    // NonceUp,
    // InitDKG,
    // SendPartMessage(Vec<u8>),
    // SendAckMessage(Vec<u8>),
    // PublicKeySetSync,
}

impl From<&theater::Message> for Event {
    fn from(msg: &theater::Message) -> Self {
        serde_json::from_slice(&msg.data).unwrap_or_default()
    }
}

impl From<theater::Message> for Event {
    fn from(msg: theater::Message) -> Self {
        serde_json::from_slice(&msg.data).unwrap_or_default()
    }
}

impl From<Vec<u8>> for Event {
    fn from(data: Vec<u8>) -> Self {
        serde_json::from_slice(&data).unwrap_or_default()
    }
}

impl From<Event> for Vec<u8> {
    fn from(evt: Event) -> Self {
        serde_json::to_vec(&evt).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
/// Contains all the potential topics.
pub enum Topic {
    Control,
    Transactions,
    State,
    Network,
}

/// EventRouter is an internal message bus that coordinates interaction
/// between runtime modules.
pub struct EventRouter {
    /// Map of async transmitters to various runtime modules
    topics: HashMap<Topic, Sender<Event>>,
}

pub type DirectedEvent = (Topic, Event);

impl Default for EventRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl EventRouter {
    pub fn new() -> Self {
        Self {
            topics: HashMap::new(),
        }
    }

    pub fn add_topic(&mut self, topic: Topic, size: Option<usize>) {
        let buffer = size.unwrap_or(1);
        let (tx, _) = broadcast::channel(buffer);

        self.topics.insert(topic, tx);
    }

    pub fn subscribe(
        &self,
        topic: &Topic,
    ) -> std::result::Result<broadcast::Receiver<Event>, Error> {
        if let Some(sender) = self.topics.get(topic) {
            Ok(sender.subscribe())
        } else {
            Err(Error::Other(format!("unable to subscribe to {topic:?}")))
        }
    }

    /// Starts the event router, distributing all incomming events to all
    /// subscribers
    pub async fn start(&mut self, event_rx: &mut UnboundedReceiver<DirectedEvent>) {
        while let Some((topic, event)) = event_rx.recv().await {
            if event == Event::Stop {
                info!("event router received stop signal");
                self.fan_out_event(Event::Stop, &topic);

                return;
            }

            self.fan_out_event(event, &topic);
        }
    }

    fn fan_out_event(&mut self, event: Event, topic: &Topic) {
        if let Some(topic_sender) = self.topics.get_mut(topic) {
            if let Err(err) = topic_sender.send(event.clone()) {
                error!("failed to send event {event:?} to topic {topic:?}: {err:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use tokio::sync::mpsc::unbounded_channel;

    use super::*;

    #[tokio::test]
    async fn should_susbcribe_to_topics() {
        let mut router = EventRouter::new();

        router.add_topic(Topic::Control, None);

        router.subscribe(&Topic::Control).unwrap();
    }

    #[tokio::test]
    async fn should_stop_when_issued_stop_event() {
        let (event_tx, mut event_rx) = unbounded_channel::<DirectedEvent>();
        let mut router = EventRouter::new();

        router.add_topic(Topic::Control, Some(10));

        let mut subscriber_rx = router.subscribe(&Topic::Control).unwrap();

        let handle = tokio::spawn(async move {
            router.start(&mut event_rx).await;
        });

        event_tx.send((Topic::Control, Event::Stop)).unwrap();

        handle.await.unwrap();

        assert_eq!(subscriber_rx.try_recv().unwrap(), Event::Stop);
    }
}

// NOTE: kept for reference
//
/// Command represents the vocabulary of available RPC-style interactions with
/// VRRB node internal components. Commands are meant to be issued by a command
/// router that controls node runtime modules.
//TODO: Review all the commands and determine which ones are needed, which can be changed
#[deprecated(note = "use Event instead")]
#[derive(Debug, Clone)]
pub enum Command {
    //TODO: Replace standard types with custom types for better readability
    // and to help engineers understand what the hell these items are.
    SendTxn(u32, String, u128), // address number, receiver address, amount
    ProcessTxn(Vec<u8>),
    ProcessTxnValidator(Vec<u8>),
    ConfirmedBlock(Vec<u8>),
    PendingBlock(Vec<u8>, String),
    InvalidBlock(Vec<u8>),
    ProcessClaim(Vec<u8>),
    CheckStateUpdateStatus((u128, Vec<u8>, u128)),
    StateUpdateCompleted(Vec<u8>),
    StoreStateDbChunk(Vec<u8>, Vec<u8>, u32, u32),
    SendState(String, u128),
    // SendMessage(SocketAddr, Message),
    GetBalance(u32),
    SendGenesis(String),
    SendStateComponents(String, Vec<u8>, String),
    GetStateComponents(String, Vec<u8>, String),
    RequestedComponents(String, Vec<u8>, String, String),
    // StoreStateComponents(Vec<u8>, ComponentTypes),
    StoreChild(Vec<u8>),
    StoreParent(Vec<u8>),
    StoreGenesis(Vec<u8>),
    StoreLedger(Vec<u8>),
    StoreNetworkState(Vec<u8>),
    // StateUpdateComponents(Vec<u8>, ComponentTypes),
    UpdateLastBlock(Vec<u8>),
    ClaimAbandoned(String, Vec<u8>),
    SlashClaims(Vec<String>),
    UpdateAppMiner(Vec<u8>),
    UpdateAppBlockchain(Vec<u8>),
    UpdateAppMessageCache(Vec<u8>),
    UpdateAppWallet(Vec<u8>),
    Publish(Vec<u8>),
    Gossip(Vec<u8>),
    AddNewPeer(String, String),
    AddKnownPeers(Vec<u8>),
    AddExplicitPeer(String, String),
    // ProcessPacket((Packet, SocketAddr)),
    Bootstrap(String, String),
    SendPing(String),
    ReturnPong(Vec<u8>, String),
    InitHandshake(String),
    ReciprocateHandshake(String, String, String),
    CompleteHandshake(String, String, String),
    ProcessAck(String, u32, String),
    CleanInbox(String),
    CheckAbandoned,
    StartMiner,
    GetHeight,
    MineBlock,
    MineGenesis,
    StopMine,
    GetState,
    ProcessBacklog,
    SendAddress,
    NonceUp,
    InitDKG,
    SendPartMessage(Vec<u8>),
    SendAckMessage(Vec<u8>),
    PublicKeySetSync,
    Stop,
    NoOp,
}
