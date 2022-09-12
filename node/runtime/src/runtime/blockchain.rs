pub fn setup_blockchain() {
    //____________________________________________________________________________________________________
    // Blockchain thread
    let mut blockchain_network_state = network_state.clone();
    let mut blockchain_reward_state = reward_state.clone();
    let blockchain_to_miner_sender = to_miner_sender.clone();
    let blockchain_to_swarm_sender = to_swarm_sender.clone();
    let blockchain_to_gossip_sender = to_gossip_tx.clone();
    let blockchain_to_blockchain_sender = to_blockchain_sender.clone();
    let blockchain_to_state_sender = to_state_sender.clone();
    let blockchain_to_app_sender = to_app_sender.clone();
    let blockchain_node_id = node_id.clone();
    std::thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let file_suffix: u32 = rng.gen();
        let mut blockchain =
            Blockchain::new(&format!("{}/test_chain_{}.db", directory, file_suffix));
        if let Err(_) = blockchain_to_app_sender
            .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
        {
            info!("Error sending blockchain update to App receiver.")
        }
        loop {
            let miner_sender = blockchain_to_miner_sender.clone();
            let swarm_sender = blockchain_to_swarm_sender.clone();
            let gossip_sender = blockchain_to_gossip_sender.clone();
            let state_sender = blockchain_to_state_sender.clone();
            let blockchain_sender = blockchain_to_blockchain_sender.clone();
            let app_sender = blockchain_to_app_sender.clone();
            // let blockchain_sender = blockchain_to_blockchain_sender.clone();
            if let Ok(command) = to_blockchain_receiver.try_recv() {
                match command {
                    Command::PendingBlock(block_bytes, sender_id) => {
                        let block = block::Block::from_bytes(&block_bytes);
                        if blockchain.updating_state {
                            blockchain
                                .future_blocks
                                .insert(block.clone().header.last_hash, block.clone());

                            if let Err(e) = app_sender
                                .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
                            {
                                info!("Error sending Blockchain to app: {:?}", e);
                            }
                        } else {
                            if let Err(e) = blockchain.process_block(
                                &blockchain_network_state,
                                &blockchain_reward_state,
                                &block,
                            ) {
                                // TODO: Replace with Command::InvalidBlock being sent to the node
                                // or gossip and being processed.
                                // If the block is invalid because of BlockOutOfSequence Error
                                // request the missing blocks Or the
                                // current state of the network (first, missing blocks later)
                                // If the block is invalid because of a NotTallestChain Error tell
                                // the miner they are missing blocks.
                                // The miner should request the current state of the network and
                                // then all the blocks they are missing.
                                match e.details {
                                    InvalidBlockErrorReason::BlockOutOfSequence => {
                                        // Stash block in blockchain.future_blocks
                                        // Request state update once. Set "updating_state" field
                                        // in blockchain to true, so that it doesn't request it on
                                        // receipt of new future blocks which will also be invalid.
                                        blockchain
                                            .future_blocks
                                            .insert(block.header.last_hash.clone(), block.clone());
                                        if !blockchain.updating_state
                                            && !blockchain.processing_backlog
                                        {
                                            // send state request and set blockchain.updating state
                                            // to true;
                                            info!("Error: {:?}", e);
                                            if let Some((_, v)) = blockchain.future_blocks.front() {
                                                let component = StateComponent::All;
                                                let message = MessageType::GetNetworkStateMessage {
                                                    sender_id: blockchain_node_id.clone(),
                                                    requested_from: sender_id.clone(),
                                                    requestor_address: addr.clone(),
                                                    requestor_node_type: node_type
                                                        .clone()
                                                        .as_bytes(),
                                                    lowest_block: v.header.block_height,
                                                    component: component.as_bytes(),
                                                };

                                                let msg_id = MessageKey::rand();
                                                let head = Header::Gossip;
                                                let gossip_msg = GossipMessage {
                                                    id: msg_id.inner(),
                                                    data: message.as_bytes(),
                                                    sender: addr.clone(),
                                                };

                                                let msg = Message {
                                                    head,
                                                    msg: gossip_msg.as_bytes().unwrap(),
                                                };

                                                let cloned_node_id = blockchain_node_id.clone();
                                                let thread_blockchain_sender =
                                                    blockchain_sender.clone();
                                                std::thread::spawn(move || {
                                                    let thread_node_id = cloned_node_id.clone();
                                                    let listener = std::net::TcpListener::bind(
                                                        "0.0.0.0:19291",
                                                    )
                                                    .unwrap();
                                                    info!("Opened TCP listener for state update");
                                                    for stream in listener.incoming() {
                                                        let loop_blockchain_sender =
                                                            thread_blockchain_sender.clone();
                                                        match stream {
                                                            Ok(mut stream) => {
                                                                info!(
                                                                    "New connection: {}",
                                                                    stream.peer_addr().unwrap()
                                                                );
                                                                let inner_node_id =
                                                                    thread_node_id.clone();
                                                                std::thread::spawn(move || {
                                                                    let stream_blockchain_sender =
                                                                        loop_blockchain_sender
                                                                            .clone();
                                                                    let mut buf = [0u8; 655360];
                                                                    let mut bytes = vec![];
                                                                    let mut total = 0;
                                                                    'reader: loop {
                                                                        let res =
                                                                            stream.read(&mut buf);
                                                                        if let Ok(size) = res {
                                                                            total += size;
                                                                            buf[0..size]
                                                                                .iter()
                                                                                .for_each(|byte| {
                                                                                    bytes.push(
                                                                                        *byte,
                                                                                    );
                                                                                });
                                                                            info!("Received total of {:?} bytes", total);
                                                                            if size == 0 {
                                                                                info!("Received all bytes, reconstructing");
                                                                                if let Some(message) = Message::from_bytes(&bytes) {
                                                                                        if let Some(gossip_msg) = GossipMessage::from_bytes(&message.msg) {
                                                                                            if let Some(message_type) = MessageType::from_bytes(&gossip_msg.data) {
                                                                                                info!("{:?}", message_type);
                                                                                                if let Some(command) = message::process_message(message_type, inner_node_id.clone(), addr.clone().to_string()) {
                                                                                                    if let Err(e) = stream_blockchain_sender.send(command) {
                                                                                                        info!("Error sending command to blockchain");
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        };
                                                                                    }
                                                                                break 'reader;
                                                                            }
                                                                        }
                                                                    }
                                                                    stream.shutdown(std::net::Shutdown::Both).expect("Unable to shutdown");
                                                                });
                                                            },
                                                            Err(e) => {},
                                                        }
                                                    }
                                                });

                                                info!("Requesting state update");
                                                if let Err(e) =
                                                    gossip_sender.send((addr.clone(), msg))
                                                {
                                                    info!("Error sending state update request to swarm sender: {:?}", e);
                                                };

                                                blockchain.updating_state = true;
                                                blockchain.started_updating =
                                                    Some(udp2p::utils::utils::timestamp_now());
                                            }
                                        }
                                    },
                                    InvalidBlockErrorReason::NotTallestChain => {
                                        // Inform the miner they are missing
                                        // blocks
                                        // info!("Error: {:?}", e);
                                    },
                                    _ => {
                                        if !blockchain.updating_state {
                                            let lowest_block = {
                                                if let Some(block) = blockchain.child.clone() {
                                                    block.clone()
                                                } else {
                                                    blockchain.genesis.clone().unwrap()
                                                }
                                            };
                                            info!("Error: {:?}", e);
                                            if block.header.block_height
                                                > lowest_block.header.block_height + 1
                                            {
                                                let component = StateComponent::All;
                                                let message = MessageType::GetNetworkStateMessage {
                                                    sender_id: blockchain_node_id.clone(),
                                                    requested_from: sender_id,
                                                    requestor_address: addr.clone(),
                                                    requestor_node_type: node_type
                                                        .clone()
                                                        .as_bytes(),
                                                    lowest_block: lowest_block.header.block_height,
                                                    component: component.as_bytes(),
                                                };

                                                let head = Header::Gossip;
                                                let msg_id = MessageKey::rand();
                                                let gossip_msg = GossipMessage {
                                                    id: msg_id.inner(),
                                                    data: message.as_bytes(),
                                                    sender: addr.clone(),
                                                };
                                                let msg = Message {
                                                    head,
                                                    msg: gossip_msg.as_bytes().unwrap(),
                                                };

                                                // TODO: Replace the below with sending to the
                                                // correct channel
                                                if let Err(e) =
                                                    gossip_sender.send((addr.clone(), msg))
                                                {
                                                    info!("Error sending state update request to swarm sender: {:?}", e);
                                                };

                                                blockchain.updating_state = true;
                                            } else {
                                                // Miner is out of consensus tell them to update
                                                // their state.
                                                let message = MessageType::InvalidBlockMessage {
                                                    block_height: block.header.block_height,
                                                    reason: e.details.as_bytes(),
                                                    miner_id: sender_id,
                                                    sender_id: blockchain_node_id.clone(),
                                                };

                                                let head = Header::Gossip;
                                                let msg_id = MessageKey::rand();
                                                let gossip_msg = GossipMessage {
                                                    id: msg_id.inner(),
                                                    data: message.as_bytes(),
                                                    sender: addr.clone(),
                                                };
                                                let msg = Message {
                                                    head,
                                                    msg: gossip_msg.as_bytes().unwrap(),
                                                };

                                                // TODO: Replace the below with sending to the
                                                // correct channel
                                                if let Err(e) =
                                                    gossip_sender.send((addr.clone(), msg))
                                                {
                                                    info!("Error sending state update request to swarm sender: {:?}", e);
                                                };

                                                blockchain
                                                    .invalid
                                                    .insert(block.hash.clone(), block.clone());
                                            }
                                        }
                                    },
                                }

                                if let Err(_) = miner_sender
                                    .send(Command::InvalidBlock(block.clone().as_bytes()))
                                {
                                    info!("Error sending command to receiver");
                                };

                                if let Err(_) = app_sender.send(Command::UpdateAppBlockchain(
                                    blockchain.clone().as_bytes(),
                                )) {
                                    info!("Error sending updated blockchain to app");
                                }
                            } else {
                                blockchain_network_state.dump(
                                    &block.txns,
                                    block.header.block_reward.clone(),
                                    &block.claims,
                                    block.header.claim.clone(),
                                    &block.hash,
                                );
                                if let Err(_) = miner_sender
                                    .send(Command::ConfirmedBlock(block.clone().as_bytes()))
                                {
                                    info!("Error sending command to receiver");
                                }

                                if let Err(_) = miner_sender.send(Command::StateUpdateCompleted(
                                    blockchain_network_state.clone().as_bytes(),
                                )) {
                                    info!(
                                        "Error sending state update completed command to receiver"
                                    );
                                }

                                if let Err(_) = app_sender.send(Command::UpdateAppBlockchain(
                                    blockchain.clone().as_bytes(),
                                )) {
                                    info!("Error sending blockchain update to App receiver.")
                                }
                            }
                        }
                    },
                    Command::GetStateComponents(requestor, components_bytes, sender_id) => {
                        info!("Received request for State update");
                        let components = StateComponent::from_bytes(&components_bytes);
                        match components {
                            StateComponent::All => {
                                let genesis_bytes =
                                    if let Some(genesis) = blockchain.clone().genesis {
                                        Some(genesis.clone().as_bytes())
                                    } else {
                                        None
                                    };
                                let child_bytes = if let Some(block) = blockchain.clone().child {
                                    Some(block.clone().as_bytes())
                                } else {
                                    None
                                };
                                let parent_bytes = if let Some(block) = blockchain.clone().parent {
                                    Some(block.clone().as_bytes())
                                } else {
                                    None
                                };
                                let current_ledger = Some(
                                    blockchain_network_state.clone().db_to_ledger().as_bytes(),
                                );
                                let current_network_state =
                                    Some(blockchain_network_state.clone().as_bytes());
                                let components = Components {
                                    genesis: genesis_bytes,
                                    child: child_bytes,
                                    parent: parent_bytes,
                                    blockchain: None,
                                    ledger: current_ledger,
                                    network_state: current_network_state,
                                    archive: None,
                                };

                                if let Err(e) = state_sender.send(Command::RequestedComponents(
                                    requestor,
                                    components.as_bytes(),
                                    sender_id.clone(),
                                    blockchain_node_id.clone(),
                                )) {
                                    info!(
                                        "Error sending requested components to state receiver: {:?}",
                                        e
                                    );
                                }
                            },
                            _ => {},
                        }
                    },
                    Command::StoreStateComponents(component_bytes, component_type) => {
                        if blockchain.updating_state {
                            blockchain
                                .components_received
                                .insert(component_type.clone());
                            match component_type {
                                ComponentTypes::All => {
                                    let components = Components::from_bytes(&component_bytes);
                                    info!("Received Components: {:?}", components);
                                    if let Some(bytes) = components.genesis {
                                        let genesis = block::Block::from_bytes(&bytes);
                                        blockchain.genesis = Some(genesis);
                                        info!("Stored Genesis: {:?}", blockchain.genesis);
                                    }
                                    if let Some(bytes) = components.child {
                                        let child = block::Block::from_bytes(&bytes);
                                        blockchain.child = Some(child);
                                        info!("Stored child: {:?}", blockchain.child);
                                    }
                                    if let Some(bytes) = components.parent {
                                        let parent = block::Block::from_bytes(&bytes);
                                        blockchain.parent = Some(parent);
                                        info!("Stored parent: {:?}", blockchain.parent);
                                    }
                                    if let Some(bytes) = components.network_state {
                                        if let Ok(mut new_network_state) =
                                            NetworkState::from_bytes(component_bytes)
                                        {
                                            new_network_state.path = blockchain_network_state.path;
                                            blockchain_reward_state =
                                                new_network_state.reward_state.unwrap();
                                            blockchain_network_state = new_network_state;
                                            info!(
                                                "Stored network state: {:?}",
                                                blockchain_network_state
                                            );
                                        }
                                    }
                                    if let Some(bytes) = components.ledger {
                                        let new_ledger = Ledger::from_bytes(bytes);
                                        blockchain_network_state.update_ledger(new_ledger);
                                        info!(
                                            "Stored ledger: {:?}",
                                            blockchain_network_state.ledger
                                        );
                                    }

                                    info!("Received all core components");
                                    blockchain.updating_state = false;
                                    if let Err(e) = blockchain_sender.send(Command::ProcessBacklog)
                                    {
                                        info!("Error sending process backlog command to blockchain receiver: {:?}", e);
                                    }
                                    blockchain.processing_backlog = true;
                                    if let Err(e) = app_sender.send(Command::UpdateAppBlockchain(
                                        blockchain.clone().as_bytes(),
                                    )) {
                                        info!("Error sending updated blockchain to app: {:?}", e);
                                    }
                                },
                                _ => {},
                            }
                        }
                    },
                    Command::ProcessBacklog => {
                        if blockchain.processing_backlog {
                            let last_block = blockchain.clone().child.unwrap();
                            while let Some((_, block)) = blockchain.future_blocks.pop_front() {
                                if last_block.header.block_height >= block.header.block_height {
                                    info!("Block already processed, skipping")
                                } else {
                                    info!(
                                        "Processing backlog block: {:?}",
                                        block.header.block_height
                                    );
                                    if let Err(e) = blockchain.process_block(
                                        &blockchain_network_state,
                                        &blockchain_reward_state,
                                        &block,
                                    ) {
                                        info!(
                                            "Error trying to process backlogged future blocks: {:?} -> {:?}",
                                            e,
                                            block,
                                        );
                                    } else {
                                        blockchain_network_state.dump(
                                            &block.txns,
                                            block.header.block_reward.clone(),
                                            &block.claims,
                                            block.header.claim.clone(),
                                            &block.hash,
                                        );
                                        info!(
                                            "Processed and confirmed backlog block: {:?}",
                                            block.header.block_height
                                        );
                                        if let Err(e) = miner_sender
                                            .send(Command::ConfirmedBlock(block.clone().as_bytes()))
                                        {
                                            info!(
                                                "Error sending confirmed backlog block to miner: {:?}",
                                                e
                                            );
                                        }

                                        if let Err(e) =
                                            app_sender.send(Command::UpdateAppBlockchain(
                                                blockchain.clone().as_bytes(),
                                            ))
                                        {
                                            info!("Error sending blockchain to app: {:?}", e);
                                        }
                                    }
                                }
                            }
                            info!("Backlog processed");

                            if let Err(e) = miner_sender.send(Command::StateUpdateCompleted(
                                blockchain_network_state.clone().as_bytes(),
                            )) {
                                info!("Error sending updated network state to miner: {:?}", e);
                            }

                            if let Err(e) = app_sender
                                .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
                            {
                                info!("Error sending updated blockchain to app: {:?}", e);
                            }
                            blockchain.processing_backlog = false;
                        }
                    },
                    Command::StateUpdateCompleted(network_state) => {
                        if let Ok(updated_network_state) = NetworkState::from_bytes(network_state) {
                            blockchain_network_state = updated_network_state;
                        }
                        if let Err(e) = app_sender
                            .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
                        {
                            info!("Error sending blockchain to app: {:?}", e);
                        }
                    },
                    Command::ClaimAbandoned(pubkey, claim_bytes) => {
                        let claim = Claim::from_bytes(&claim_bytes);
                        blockchain_network_state.abandoned_claim(claim.hash.clone());
                        if let Err(_) =
                            miner_sender.send(Command::ClaimAbandoned(pubkey, claim_bytes))
                        {
                            info!("Error sending claim abandoned command to miner");
                        }
                        if let Err(e) = miner_sender.send(Command::StateUpdateCompleted(
                            blockchain_network_state.clone().as_bytes(),
                        )) {
                            info!("Error sending updated network state to miner: {:?}", e);
                        }

                        if let Err(e) = app_sender
                            .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
                        {
                            info!("Error sending blockchain to app: {:?}", e);
                        }
                    },
                    Command::SlashClaims(bad_validators) => {
                        blockchain_network_state.slash_claims(bad_validators);
                        if let Err(e) = app_sender
                            .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
                        {
                            info!("Error sending blockchain to app: {:?}", e);
                        }
                    },
                    Command::NonceUp => {
                        blockchain_network_state.nonce_up();
                    },
                    Command::GetHeight => {
                        info!("Blockchain Height: {}", blockchain.chain.len());
                    },
                    _ => {},
                }
            }
        }
    });
}
