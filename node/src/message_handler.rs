use std::{net::SocketAddr, sync::mpsc::Sender};

/// This module is the primary allocator in the system, it contains the data
/// structures and the methods required to send commands to different parts of
/// the system.
use commands::command::Command;
use telemetry::info;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use udp2p::protocol::protocol::Message;

use crate::handler::Handler;

/// The basic structure for allocating messages to the transport layer, and
/// receiving messages to be converted into commands from the transport layer.
#[derive(Debug)]
pub struct MessageHandler<T, V> {
    pub sender: UnboundedSender<T>,
    pub receiver: UnboundedReceiver<V>,
}

impl<T: Clone, V: Clone> MessageHandler<T, V> {
    /// Creates and returns a new message handler.
    pub fn new(sender: UnboundedSender<T>, receiver: UnboundedReceiver<V>) -> MessageHandler<T, V> {
        MessageHandler { sender, receiver }
    }
}

impl<T: Clone, V: Clone> Handler<T, V> for MessageHandler<T, V> {
    fn send(&self, command: T) -> Option<T> {
        if let Err(_) = self.sender.send(command.clone()) {
            return None;
        } else {
            return Some(command);
        }
    }

    fn recv(&mut self) -> Option<V> {
        if let Ok(message) = self.receiver.try_recv() {
            return Some(message);
        } else {
            return None;
        }
    }
}
