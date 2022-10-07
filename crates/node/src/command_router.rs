use std::collections::HashMap;

use commands::command::Command;
use tokio::sync::mpsc::{error::TryRecvError, UnboundedReceiver, UnboundedSender};

<<<<<<< HEAD
use crate::Result;
=======
use crate::{NodeError, Result};
>>>>>>> 67c70bb (reorganize crates)

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
/// Contains all the potential destinations a command can be issued to
pub enum CommandRoute {
    Blockchain,
    Swarm,
    State,
    Miner,
    Router,
}

pub type Subscriber = UnboundedSender<Command>;
<<<<<<< HEAD
=======
pub type CommandPublisher = UnboundedSender<DirectedCommand>;
>>>>>>> 67c70bb (reorganize crates)

/// CommandRouter is an internal message bus that coordinates interaction
/// between runtime modules
pub struct CommandRouter {
    /// Map of async transmitters to various runtime modules
    subscribers: HashMap<CommandRoute, Subscriber>,
<<<<<<< HEAD
=======
    // command_rx: UnboundedReceiver<DirectedCommand>,
>>>>>>> 67c70bb (reorganize crates)
}

pub type DirectedCommand = (CommandRoute, Command);

impl CommandRouter {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
        }
    }

    pub fn add_subscriber(&mut self, key: CommandRoute, subscriber: Subscriber) -> Result<()> {
        // TODO; add safety checks to avoid orphaned subscribers
        self.subscribers.insert(key, subscriber);

        Ok(())
    }

<<<<<<< HEAD
=======
    // pub fn start(&mut self) -> Result<()> {
>>>>>>> 67c70bb (reorganize crates)
    pub async fn start(
        &mut self,
        command_rx: &mut UnboundedReceiver<DirectedCommand>,
    ) -> Result<()> {
        return Ok(());

        loop {
<<<<<<< HEAD
=======
            // let cmd = match self.command_rx.try_recv() {
>>>>>>> 67c70bb (reorganize crates)
            let cmd = match command_rx.try_recv() {
                Ok(cmd) => cmd,
                Err(err) if err == TryRecvError::Disconnected => {
                    //TODO: refactor this error handling
                    //TODO: log this stop condition
                    (CommandRoute::Router, Command::Stop)
                },
                // TODO: log all other errors
                _ => (CommandRoute::Router, Command::NoOp),
            };

            match cmd {
                (_, Command::Stop) => {
                    //TODO: forward stop command to all subscribers
<<<<<<< HEAD
=======
                    for (_, sub) in self.subscribers {
                        if let Err(err) = sub.send(Command::Stop) {
                            return Err(NodeError::Other(err.to_string()));
                        }
                        // TODO: trace log success
                    }

>>>>>>> 67c70bb (reorganize crates)
                    break;
                },
                (_, cmd) => {
                    telemetry::warn!("Unrecognized command received: {:?}", cmd);
                },
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn should_register_susbcribers() {
        let (_, mut command_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedCommand>();
<<<<<<< HEAD
=======
        // let mut router = CommandRouter::new(command_rx);
>>>>>>> 67c70bb (reorganize crates)
        let mut router = CommandRouter::new();

        let (miner_command_tx, mut miner_command_rx) =
            tokio::sync::mpsc::unbounded_channel::<Command>();

        router
            .add_subscriber(CommandRoute::Miner, miner_command_tx)
            .unwrap();
    }

    #[tokio::test]
    async fn should_stop_when_issued_stop_command() {
        let (command_tx, mut command_rx) =
            tokio::sync::mpsc::unbounded_channel::<DirectedCommand>();
        let mut router = CommandRouter::new();

        let handle = tokio::spawn(async move {
<<<<<<< HEAD
=======
            // router.start(&mut command_rx).await.unwrap();
>>>>>>> 67c70bb (reorganize crates)
            router.start(&mut command_rx).await.unwrap();
        });

        command_tx
            .send((CommandRoute::Router, Command::Stop))
            .unwrap();

        handle.await.unwrap();
    }

    #[tokio::test]
    async fn should_route_commands() {
        let (command_tx, mut command_rx) =
            tokio::sync::mpsc::unbounded_channel::<DirectedCommand>();
        let mut router = CommandRouter::new();

        let (miner_command_tx, mut miner_command_rx) =
            tokio::sync::mpsc::unbounded_channel::<Command>();

        router
            .add_subscriber(CommandRoute::Miner, miner_command_tx)
            .unwrap();

        let handle = tokio::spawn(async move {
<<<<<<< HEAD
=======
            // router.start(&mut command_rx).await.unwrap();
>>>>>>> 67c70bb (reorganize crates)
            router.start(&mut command_rx).await.unwrap();
        });

        command_tx
            .send((CommandRoute::Router, Command::Stop))
            .unwrap();

        handle.await.unwrap();
    }
}
