use std::collections::HashMap;

use commands::command::Command;
use tokio::sync::mpsc::{error::TryRecvError, UnboundedReceiver, UnboundedSender};

use crate::Result;

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

/// CommandRouter is an internal message bus that coordinates interaction
/// between runtime modules
pub struct CommandRouter {
    /// Map of async transmitters to various runtime modules
    subscribers: HashMap<CommandRoute, Subscriber>,
    // command_rx: UnboundedReceiver<DirectedCommand>,
}

pub type DirectedCommand = (CommandRoute, Command);

impl CommandRouter {
    // pub fn new(command_rx: UnboundedReceiver<DirectedCommand>) -> Self {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
            // command_rx,
        }
    }

    pub fn add_subscriber(&mut self, key: CommandRoute, subscriber: Subscriber) -> Result<()> {
        // TODO; add safety checks to avoid orphaned subscribers
        self.subscribers.insert(key, subscriber);

        Ok(())
    }

    // pub fn start(&mut self) -> Result<()> {
    pub async fn start(
        &mut self,
        command_rx: &mut UnboundedReceiver<DirectedCommand>,
    ) -> Result<()> {
        return Ok(());

        loop {
            // let cmd = match self.command_rx.try_recv() {
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
        // let mut router = CommandRouter::new(command_rx);
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
            // router.start(&mut command_rx).await.unwrap();
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
            // router.start(&mut command_rx).await.unwrap();
            router.start(&mut command_rx).await.unwrap();
        });

        command_tx
            .send((CommandRoute::Router, Command::Stop))
            .unwrap();

        handle.await.unwrap();
    }
}
