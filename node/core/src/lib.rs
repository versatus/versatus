pub mod command_handler;
pub mod message_handler;

pub mod handler;
#[deprecated(note = "use node::core instead")]
pub mod node;

pub mod result;
pub use result::*;

pub mod core {
    // TODO rename node.rs to core.rs once other refactoring efforts are complete
    pub use super::node::*;
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, sync::mpsc::Sender, thread};

    use commands::command::Command;
    use log::info;
    use messages::{message_types::MessageType, packet::Packet};
    use telemetry::TelemetrySubscriber;
    use tokio::sync::mpsc;

    use crate::{
        command_handler::CommandHandler,
        core::{Node, NodeType},
        handler::Handler,
        message_handler::MessageHandler,
    };

    #[tokio::test]
    async fn node_can_receive_start_and_stop() {
        let (mining_sender, mining_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut mining_handler = MessageHandler::new(mining_sender, mining_receiver);

        let (blockchain_sender, blockchain_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

        let (gossip_sender, gossip_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

        let (swarm_sender, swarm_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

        let (state_sender, mut state_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let state_handler = MessageHandler::new(state_sender, state_receiver);

        let (gossip_tx_sender, gossip_tx_receiver) = std::sync::mpsc::channel();

        let (ctrl_sender, mut ctrl_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let ctrl_handler = MessageHandler::new(ctrl_sender.clone(), ctrl_receiver);

        let mut cmd_handler = CommandHandler::new(
            mining_handler.sender,
            blockchain_handler.sender,
            gossip_handler.sender,
            swarm_handler.sender,
            state_handler.sender,
            gossip_tx_sender,
            ctrl_handler.receiver,
        );

        // NOTE: in case logging is needed, remove if not needed
        TelemetrySubscriber::init(std::io::stdout).unwrap();

        let (msg_sender, mut msg_receiver) = tokio::sync::mpsc::unbounded_channel::<MessageType>();
        let (msg_sender_1, mut msg_receiver_1) =
            tokio::sync::mpsc::unbounded_channel::<(Packet, SocketAddr)>();

        let msg_handler = MessageHandler::new(msg_sender, msg_receiver_1);

        // NOTE: preliminary setup above this note
        let mut node = Node::new(NodeType::Full, cmd_handler, msg_handler, 100);

        let handle = tokio::spawn(async move {
            node.start().await.unwrap();
        });

        ctrl_sender.send(Command::Stop).unwrap();

        // NOTE: stop the node after tests are performed
        ctrl_sender.send(Command::Stop).unwrap();
        handle.await.unwrap();
    }
}
