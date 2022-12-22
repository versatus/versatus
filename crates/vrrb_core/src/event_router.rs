use std::collections::{hash_map::Entry, HashMap};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::Error;

pub type Subscriber = UnboundedSender<Event>;
pub type Publisher = UnboundedSender<(Topic, Event)>;

// NOTE: naming convention for events goes as follows:
// <Subject><Verb, in past tense>, e.g. ObjectCreated
// TODO: Replace Vec<u8>'s with proper data structs in enum wariants
// once definitions of those are moved into primitives.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Event {
    NoOp,
    Stop,
    /// New txn came from network, requires validation
    TxnCreated(Vec<u8>),
    /// Batch of validated txns
    TxnBatchValidated(Vec<u8>),
    BlockConfirmed(Vec<u8>),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
/// Contains all the potential topics.
pub enum Topic {
    Control,
    Transactions,
    State,
}

/// EventRouter is an internal message bus that coordinates interaction
/// between runtime modules.
pub struct EventRouter {
    /// Map of async transmitters to various runtime modules
    subscribers: HashMap<Topic, Vec<Subscriber>>,
    topics: HashMap<Topic, tokio::sync::broadcast::Sender<Event>>,
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
            subscribers: HashMap::new(),
            topics: HashMap::new(),
        }
    }

    pub fn add_topic(&mut self, topic: Topic, size: Option<usize>) {
        let buffer = size.unwrap_or(1);
        let (tx, _) = tokio::sync::broadcast::channel(buffer);

        self.topics.insert(topic, tx);
    }

    pub fn subscribe(
        &self,
        topic: &Topic,
    ) -> std::result::Result<tokio::sync::broadcast::Receiver<Event>, Error> {
        if let Some(sender) = self.topics.get(topic) {
            Ok(sender.subscribe())
        } else {
            Err(Error::Other(format!("unable to subscribe to {topic:?}")))
        }
    }

    #[deprecated(note = "replaced by 'subscribe'")]
    pub fn add_subscriber(&mut self, topic: Topic, subscriber: Subscriber) {
        match self.subscribers.entry(topic) {
            Entry::Occupied(mut subscribers) => subscribers.get_mut().push(subscriber),
            Entry::Vacant(empty) => {
                empty.insert(vec![subscriber]);
            },
        }
    }

    /// Starts the event router, distributing all incomming commands to
    /// specified routes
    pub async fn start(&mut self, event_rx: &mut UnboundedReceiver<DirectedEvent>) {
        while let Some((topic, event)) = event_rx.recv().await {
            if event == Event::Stop {
                telemetry::info!("event router received stop signal");
                self.fan_out_event(Event::Stop, &topic);

                break;
            }

            self.fan_out_event(event, &topic);
        }
    }

    fn fan_out_event(&mut self, event: Event, topic: &Topic) {
        if let Some(subscriber_list) = self.subscribers.get_mut(topic) {
            for subscriber in subscriber_list {
                //TODO: report errors
                if let Err(err) = subscriber.send(event.clone()) {
                    telemetry::error!(
                        "failed to send event {:?} to topic {:?}. reason: {:?}",
                        event,
                        topic,
                        err
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use tokio::sync::mpsc::unbounded_channel;

    use super::*;

    #[test]
    fn should_register_susbcribers() {
        let mut router = EventRouter::new();

        let (miner_command_tx, _) = unbounded_channel::<Event>();

        router.add_subscriber(Topic::Control, miner_command_tx);

        let control_subscribers = router.subscribers.get(&Topic::Control).unwrap();
        assert_eq!(control_subscribers.len(), 1);
    }

    #[tokio::test]
    async fn should_stop_when_issued_stop_event() {
        let (event_tx, mut event_rx) = unbounded_channel::<DirectedEvent>();
        let (subscriber_tx, mut subscriber_rx) = unbounded_channel::<Event>();

        let mut router = EventRouter::new();

        router.add_subscriber(Topic::Control, subscriber_tx);

        let handle = tokio::spawn(async move {
            router.start(&mut event_rx).await;
        });

        event_tx.send((Topic::Control, Event::Stop)).unwrap();

        handle.await.unwrap();

        assert_eq!(subscriber_rx.try_recv().unwrap(), Event::Stop);
    }

    #[tokio::test]
    async fn should_route_events() {
        let (event_tx, mut event_rx) = unbounded_channel::<DirectedEvent>();
        let mut router = EventRouter::new();

        let (miner_event_tx, mut miner_event_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
        let (validator_event_tx, mut validator_event_rx) = unbounded_channel::<Event>();

        router.add_subscriber(Topic::Control, miner_event_tx);
        router.add_subscriber(Topic::Control, validator_event_tx.clone());
        router.add_subscriber(Topic::Transactions, validator_event_tx);

        let handle = tokio::spawn(async move {
            router.start(&mut event_rx).await;
        });

        event_tx
            .send((Topic::Transactions, Event::TxnCreated(Vec::new())))
            .unwrap();

        event_tx.send((Topic::Control, Event::Stop)).unwrap();

        handle.await.unwrap();

        assert_eq!(
            validator_event_rx.recv().await.unwrap(),
            Event::TxnCreated(Vec::new())
        );

        assert_eq!(validator_event_rx.recv().await.unwrap(), Event::Stop);
        assert_eq!(miner_event_rx.recv().await.unwrap(), Event::Stop);
    }

    #[tokio::test]
    async fn all_subscribers_should_receive_messages() {
        let (event_tx, mut event_rx) = unbounded_channel::<DirectedEvent>();
        let mut router = EventRouter::new();

        let (miner_event_tx, mut miner_event_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
        let (validator_event_tx, mut validator_event_rx) = unbounded_channel::<Event>();

        router.add_subscriber(Topic::Control, miner_event_tx);
        router.add_subscriber(Topic::Control, validator_event_tx.clone());
        router.add_subscriber(Topic::Transactions, validator_event_tx);

        let handle = tokio::spawn(async move {
            router.start(&mut event_rx).await;
        });

        event_tx
            .send((Topic::Transactions, Event::TxnCreated(Vec::new())))
            .unwrap();

        event_tx.send((Topic::Control, Event::Stop)).unwrap();

        handle.await.unwrap();

        assert_eq!(
            validator_event_rx.recv().await.unwrap(),
            Event::TxnCreated(Vec::new())
        );

        assert_eq!(validator_event_rx.recv().await.unwrap(), Event::Stop);
        assert_eq!(miner_event_rx.recv().await.unwrap(), Event::Stop);
    }
}

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
