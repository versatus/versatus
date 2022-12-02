pub struct StateSyncReporter;
impl StateSyncReporter {
    /// Creates and sends (to transport layer channel for sending to
    /// network/miner) a message in the event of an invalid block
    /// to inform the miner that they proposed an invalid block.
    pub fn send_invalid_block_message(
        &self,
        block: &Block,
        reason: InvalidBlockErrorReason,
        miner_id: String,
        sender_id: String,
        gossip_tx: std::sync::mpsc::Sender<(SocketAddr, Message)>,
        src: SocketAddr,
    ) {
        let message = MessageType::InvalidBlockMessage {
            block_height: block.clone().header.block_height,
            reason: reason.as_bytes(),
            miner_id,
            sender_id,
        };
        let msg_id = MessageKey::rand();
        let gossip_msg = GossipMessage {
            id: msg_id.inner(),
            data: message.as_bytes(),
            sender: src,
        };
        let head = Header::Gossip;
        let msg = Message {
            head,
            msg: gossip_msg.as_bytes().unwrap(),
        };

        if let Err(e) = gossip_tx.send((src, msg)) {
            println!(
                "Error sending InvalidBlockMessage InvalidBlockHeight to swarm sender: {:?}",
                e
            );
        }
    }

    /// Checks if all state core components have been received
    /// Core components include:
    ///     Genesis Block
    ///     Child (Last) Block
    ///     Parent (Previous block to child) Block
    ///     The current state of the network
    ///     The current network ledger
    pub fn received_core_components(&self) -> bool {
        self.components_received.contains(&ComponentTypes::Genesis)
            && self.components_received.contains(&ComponentTypes::Child)
            && self.components_received.contains(&ComponentTypes::Parent)
            && self
                .components_received
                .contains(&ComponentTypes::NetworkState)
            && self.components_received.contains(&ComponentTypes::Ledger)
    }

    /// Checks how long since the request was sent for state update
    pub fn check_time_since_update_request(&self) -> Option<u128> {
        let now = timestamp_now();
        if let Some(time) = self.started_updating {
            let diff = now.checked_sub(time);
            info!("Time in nanos since last update: {:?}", diff);
            diff
        } else {
            None
        }
    }

    /// Resends the request to update state if too much time has passed
    pub fn request_again(&self) -> bool {
        if let Some(nanos) = self.check_time_since_update_request() {
            nanos > 1000000000
        } else {
            false
        }
    }
}
