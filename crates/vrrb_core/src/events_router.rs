use std::collections::{hash_map::Entry, HashMap};

// use telemetry::tracing::subscriber;
use tokio::sync::mpsc::{error::TryRecvError, UnboundedReceiver, UnboundedSender};

pub type Subscriber = UnboundedSender<Event>;
pub type Publisher = UnboundedSender<(Topic, Event)>;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Event {
    Stop,
    Start,
    NewTxn(Vec<u8>),            // New txn came from network, requires validation
    ValidatedTxnBatch(Vec<u8>), // Batch of validated txns
    NewConfirmedBlock(Vec<u8>),
    NoOp,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
/// Contains all the potential topics a Event may be tied to
pub enum Topic {
    Control,
    Transactions,
}

/// CommandRouter is an internal message bus that coordinates interaction
/// between runtime modules. It's a generic version of CommandHandler
pub struct EventRouter {
    /// Map of async transmitters to various runtime modules
    subscribers: HashMap<Topic, Vec<Subscriber>>,
}

pub type DirectedEvent = (Topic, Event);

impl EventRouter {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
        }
    }

    pub fn add_subscriber(&mut self, topic: Topic, subscriber: Subscriber) {
        match self.subscribers.entry(topic) {
            Entry::Occupied(mut subscribers) => subscribers.get_mut().push(subscriber),
            Entry::Vacant(empty) => {
                empty.insert(vec![subscriber]);
            }
        }
    }

    /// Starts the command router, distributing all incomming commands to
    /// specified routes
    pub async fn start(&mut self, command_rx: &mut UnboundedReceiver<DirectedEvent>) {
        loop {
            let cmd = match command_rx.try_recv() {
                Ok(cmd) => cmd,
                Err(err) if err == TryRecvError::Disconnected => {
                    telemetry::error!("The command channel for command router has been closed. ");
                    (Topic::Control, Event::Stop)
                    //TODO: refactor this error handling
                }
                // TODO: log all other errors
                _ => (Topic::Control, Event::NoOp),
            };

            if let Some(subscriber_list) = self.subscribers.get_mut(&cmd.0) {
                for subscriber in subscriber_list.clone() {
                    if let Err(err) = subscriber.send(cmd.1.clone()) {
                        // TODO: Think about if we should drop the subscriber only from that topic,
                        // or from all Remove subscriber from that topic,
                        // since their channel is closed
                        subscriber_list.retain(|sub| !sub.same_channel(&subscriber));
                        telemetry::error!("{:?}", err);
                    }
                }
            };

            if cmd.1 == Event::Stop {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use tokio::sync::mpsc::unbounded_channel;

    #[test]
    fn should_register_susbcribers() {
        let mut router = EventRouter::new();

        let (miner_command_tx, _) = unbounded_channel::<Event>();

        router.add_subscriber(Topic::Control, miner_command_tx);

        let control_subscribers = router.subscribers.get(&Topic::Control).unwrap();
        assert_eq!(control_subscribers.len(), 1);
    }

    #[tokio::test]
    async fn should_stop_when_issued_stop_command() {
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
    async fn should_route_commands() {
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

        event_tx.send((Topic::Control, Event::Start)).unwrap();
        event_tx
            .send((Topic::Transactions, Event::NewTxn(Vec::new())))
            .unwrap();
        event_tx.send((Topic::Control, Event::Stop)).unwrap();

        handle.await.unwrap();

        assert_eq!(validator_event_rx.recv().await.unwrap(), Event::Start);
        assert_eq!(
            validator_event_rx.recv().await.unwrap(),
            Event::NewTxn(Vec::new())
        );
        assert_eq!(validator_event_rx.recv().await.unwrap(), Event::Stop);

        assert_eq!(miner_event_rx.recv().await.unwrap(), Event::Start);
        assert_eq!(miner_event_rx.recv().await.unwrap(), Event::Stop);
    }
}
