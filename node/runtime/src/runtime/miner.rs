use crate::{result::Result, RuntimeModule, RuntimeModuleState};

pub struct MiningModule {
    //
}

impl MiningModule {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuntimeModule for MiningModule {
    fn name(&self) -> String {
        todo!()
    }

    fn status(&self) -> RuntimeModuleState {
        todo!()
    }

    fn start(&self) -> Result<()> {
        todo!()
    }

    fn stop(&self) -> Result<()> {
        todo!()
    }

    fn force_stop(&self) {
        todo!()
    }
}

/*
fn setup_mining() {
    let mining_wallet = wallet.clone();
    let miner_network_state = network_state.clone();
    let miner_reward_state = reward_state.clone();
    let miner_to_miner_sender = to_miner_sender.clone();
    let miner_to_blockchain_sender = to_blockchain_sender.clone();
    let miner_to_gossip_sender = to_gossip_tx.clone();
    let miner_to_app_sender = to_app_sender.clone();
    let miner_node_id = node_id.clone();
    std::thread::spawn(move || {
        let mut miner = Miner::start(
            mining_wallet.clone().get_secretkey(),
            mining_wallet.clone().get_pubkey(),
            mining_wallet.clone().get_address(1),
            miner_reward_state,
            miner_network_state,
            0,
        );
        if let Err(_) = miner_to_app_sender
            .clone()
            .send(Command::UpdateAppMiner(miner.as_bytes()))
        {
            info!("Error sending miner to app");
        }
        loop {
            let blockchain_sender = miner_to_blockchain_sender.clone();
            let gossip_sender = miner_to_gossip_sender.clone();
            let miner_sender = miner_to_miner_sender.clone();
            let app_sender = miner_to_app_sender.clone();
            if let Ok(command) = to_miner_receiver.try_recv() {
                match command {
                    Command::SendMessage(src, message) => {
                        // TODO: Replace the below with sending to the correct channel
                        if let Err(e) = gossip_sender.send((src, message)) {
                            info!("Error sending to swarm receiver: {:?}", e);
                        }
                    },
                    Command::StartMiner => {
                        miner.mining = true;
                        if let Err(_) = miner_sender.send(Command::MineBlock) {
                            info!("Error sending mine block command to miner");
                        }
                    },
                    Command::MineBlock => {
                        if miner.mining {
                            if let Some(last_block) = miner.last_block.clone() {
                                if let Some(claim) =
                                    miner.clone().claim_map.get(&miner.clone().claim.pubkey)
                                {
                                    let lowest_pointer = miner.get_lowest_pointer(
                                        last_block.header.next_block_nonce as u128,
                                    );
                                    if let Some((hash, _)) = lowest_pointer.clone() {
                                        if hash == claim.hash.clone() {
                                            let block = miner.mine();
                                            if let Some(block) = block {
                                                let message = MessageType::BlockMessage {
                                                    block: block.clone().as_bytes(),
                                                    sender_id: miner_node_id.clone().to_string(),
                                                };

                                                let msg_id = MessageKey::rand();
                                                let gossip_msg = GossipMessage {
                                                    id: msg_id.inner(),
                                                    data: message.as_bytes(),
                                                    sender: addr.clone(),
                                                };

                                                let head = Header::Gossip;

                                                let msg = Message {
                                                    head,
                                                    msg: gossip_msg.as_bytes().unwrap(),
                                                };

                                                miner.mining = false;

                                                // TODO: Replace the below with sending to the
                                                // correct channel
                                                if let Err(e) =
                                                    gossip_sender.send((addr.clone(), msg))
                                                {
                                                    info!("Error sending SendMessage command to swarm: {:?}", e);
                                                }

                                                if let Err(_) =
                                                    blockchain_sender.send(Command::PendingBlock(
                                                        block.clone().as_bytes(),
                                                        miner_node_id.clone().to_string(),
                                                    ))
                                                {
                                                    info!("Error sending PendingBlock command to blockchain");
                                                }
                                            } else {
                                                if let Err(e) =
                                                    miner_sender.send(Command::MineBlock)
                                                {
                                                    info!(
                                                        "Error sending miner sender MineBlock: {:?}",
                                                        e
                                                    );
                                                }
                                            }
                                        } else {
                                            miner.mining = false;
                                            if let Err(_) =
                                                miner_sender.send(Command::CheckAbandoned)
                                            {
                                                info!("Error sending check abandoned command to miner");
                                            }
                                        }
                                    } else {
                                        if let Err(e) = miner_sender.send(Command::NonceUp) {
                                            info!(
                                                "Error sending NonceUp command to miner: {:?}",
                                                e
                                            );
                                        }
                                    }
                                }
                            } else {
                                if let Err(e) = miner_sender.send(Command::MineGenesis) {
                                    info!("Error sending mine genesis command to miner: {:?}", e);
                                };
                            }
                        }
                    },
                    Command::ConfirmedBlock(block_bytes) => {
                        let block = block::Block::from_bytes(&block_bytes);
                        miner.current_nonce_timer = block.header.timestamp;

                        if let Category::Motherlode(_) = block.header.block_reward.category {
                            info!("*****{:?}*****\n", &block.header.block_reward.category);
                        }
                        miner.last_block = Some(block.clone());
                        block.txns.iter().for_each(|(k, _)| {
                            miner.txn_pool.confirmed.remove(&k.clone());
                        });
                        let mut new_claims = block.claims.clone();
                        new_claims = new_claims
                            .iter()
                            .map(|(k, v)| {
                                return (k.clone(), v.clone());
                            })
                            .collect();
                        new_claims.iter().for_each(|(k, v)| {
                            miner.claim_pool.confirmed.remove(k);
                            miner.claim_map.insert(k.clone(), v.clone());
                        });

                        // Check if the miner's claim nonce changed,
                        // if it did change, make sure that it HAD to change.
                        // If it did have to change (nonce up) and your local claim map is different
                        // nonce up the local claim map until it is in consensus.
                        miner.claim_map.replace(
                            block.header.claim.clone().pubkey,
                            block.header.claim.clone(),
                        );

                        if let Err(_) = app_sender.send(Command::UpdateAppMiner(miner.as_bytes())) {
                            info!("Error sending updated miner to app")
                        }
                    },
                    Command::ProcessTxn(txn_bytes) => {
                        let txn = Txn::from_bytes(&txn_bytes);
                        let txn_validator = miner.process_txn(txn.clone());
                        miner.check_confirmed(txn.txn_id.clone());
                        let message = MessageType::TxnValidatorMessage {
                            txn_validator: txn_validator.as_bytes(),
                            sender_id: miner_node_id.clone(),
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

                        // TODO: Replace the below with sending to the correct channel
                        if let Err(e) = gossip_sender.send((addr.clone(), msg)) {
                            info!("Error sending SendMessage command to swarm: {:?}", e);
                        }
                        if let Err(_) = app_sender.send(Command::UpdateAppMiner(miner.as_bytes())) {
                            info!("Error sending updated miner to app.")
                        }
                    },
                    Command::ProcessClaim(claim_bytes) => {
                        let claim = Claim::from_bytes(&claim_bytes);
                        miner
                            .claim_pool
                            .confirmed
                            .insert(claim.pubkey.clone(), claim.clone());
                        if let Err(_) = app_sender.send(Command::UpdateAppMiner(miner.as_bytes())) {
                            info!("Error sending updated miner to app")
                        }
                    },
                    Command::ProcessTxnValidator(validator_bytes) => {
                        let validator = TxnValidator::from_bytes(&validator_bytes);
                        miner.process_txn_validator(validator.clone());
                        if let Some(bad_validators) =
                            miner.check_rejected(validator.txn.txn_id.clone())
                        {
                            if let Err(e) =
                                blockchain_sender.send(Command::SlashClaims(bad_validators.clone()))
                            {
                                info!(
                                    "Error sending SlashClaims command to blockchain thread: {:?}",
                                    e
                                );
                            }

                            bad_validators.iter().for_each(|k| {
                                miner.slash_claim(k.to_string());
                            });
                        } else {
                            miner.check_confirmed(validator.txn.txn_id.clone());
                        }

                        if let Err(_) = app_sender.send(Command::UpdateAppMiner(miner.as_bytes())) {
                            info!("Error sending updated miner to app")
                        }
                    },
                    Command::InvalidBlock(_) => {
                        if let Err(e) = miner_sender.send(Command::MineBlock) {
                            info!("Error sending mine block command to miner: {:?}", e);
                        }
                    },
                    Command::StateUpdateCompleted(network_state_bytes) => {
                        if let Ok(updated_network_state) =
                            NetworkState::from_bytes(network_state_bytes)
                        {
                            miner.network_state = updated_network_state.clone();
                            miner.claim_map = miner.network_state.get_claims();
                            miner.mining = true;
                            if let Err(e) = miner_sender.send(Command::MineBlock) {
                                info!("Error sending MineBlock command to miner: {:?}", e);
                            }
                            if let Err(_) =
                                app_sender.send(Command::UpdateAppMiner(miner.as_bytes()))
                            {
                                info!("Error sending updated miner to app")
                            }
                        }
                    },
                    Command::MineGenesis => {
                        if let Some(block) = miner.genesis() {
                            miner.mining = false;
                            miner.last_block = Some(block.clone());
                            let message = MessageType::BlockMessage {
                                block: block.clone().as_bytes(),
                                sender_id: miner_node_id.clone(),
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
                            // TODO: Replace the below with sending to the correct channel
                            if let Err(e) = gossip_sender.send((addr.clone(), msg)) {
                                info!("Error sending SendMessage command to swarm: {:?}", e);
                            }
                            if let Err(_) = blockchain_sender.send(Command::PendingBlock(
                                block.clone().as_bytes(),
                                miner_node_id.clone(),
                            )) {
                                info!("Error sending to command receiver")
                            }
                            if let Err(_) =
                                app_sender.send(Command::UpdateAppMiner(miner.as_bytes()))
                            {
                                info!("Error sending updated miner to app")
                            }
                        }
                    },
                    Command::SendAddress => {
                        let message = MessageType::ClaimMessage {
                            claim: miner.claim.clone().as_bytes(),
                            sender_id: miner_node_id.clone(),
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

                        // TODO: Replace the below with sending to the correct channel
                        if let Err(e) = gossip_sender.send((addr.clone(), msg)) {
                            info!("Error sending SendMessage command to swarm: {:?}", e);
                        }
                    },
                    Command::NonceUp => {
                        miner.nonce_up();
                        if let Err(e) = blockchain_sender.send(Command::NonceUp) {
                            info!("Error sending NonceUp command to blockchain: {:?}", e);
                        }
                        if let Err(_) = app_sender.send(Command::UpdateAppMiner(miner.as_bytes())) {
                            info!("Error sending updated miner to app")
                        }
                        if let Err(e) = miner_sender.send(Command::MineBlock) {
                            info!("Error sending MineBlock command to miner: {:?}", e);
                        }
                    },
                    Command::CheckAbandoned => {
                        if let Some(last_block) = miner.last_block.clone() {
                            if let Some(_) =
                                miner.clone().claim_map.get(&miner.clone().claim.pubkey)
                            {
                                let lowest_pointer = miner
                                    .get_lowest_pointer(last_block.header.next_block_nonce as u128);
                                if let Some((hash, _)) = lowest_pointer.clone() {
                                    if miner.check_time_elapsed() > 30 {
                                        miner.current_nonce_timer = miner.get_timestamp();
                                        let mut abandoned_claim_map = miner.claim_map.clone();
                                        abandoned_claim_map.retain(|_, v| v.hash == hash);

                                        if let Some((_, v)) = abandoned_claim_map.front() {
                                            let message = MessageType::ClaimAbandonedMessage {
                                                claim: v.clone().as_bytes(),
                                                sender_id: miner_node_id.clone(),
                                            };

                                            miner
                                                .abandoned_claim_counter
                                                .insert(miner.claim.pubkey.clone(), v.clone());

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
                                            // TODO: Replace the below with sending to the correct
                                            // channel
                                            if let Err(e) = gossip_sender.send((addr.clone(), msg))
                                            {
                                                info!("Error sending ClaimAbandoned message to swarm: {:?}", e);
                                            }

                                            let mut abandoned_claim_map =
                                                miner.abandoned_claim_counter.clone();

                                            abandoned_claim_map
                                                .retain(|_, claim| v.hash == claim.hash);

                                            if abandoned_claim_map.len() as f64
                                                / (miner.claim_map.len() as f64 - 1.0)
                                                > VALIDATOR_THRESHOLD
                                            {
                                                miner.claim_map.retain(|_, v| v.hash != hash);
                                                if let Err(e) =
                                                    blockchain_sender.send(Command::ClaimAbandoned(
                                                        miner.claim.pubkey.clone(),
                                                        v.clone().as_bytes(),
                                                    ))
                                                {
                                                    info!("Error forwarding confirmed abandoned claim to blockchain: {:?}", e);
                                                }
                                            }
                                        }
                                    } else {
                                        if let Err(_) = miner_sender.send(Command::CheckAbandoned) {
                                            info!("Error sending check abandoned command to miner");
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Command::ClaimAbandoned(pubkey, _) => {
                        if let Some(claim) = miner.claim_map.clone().get(&pubkey) {
                            miner
                                .abandoned_claim_counter
                                .insert(pubkey.clone(), claim.clone());

                            let mut abandoned_claim_map = miner.abandoned_claim_counter.clone();
                            abandoned_claim_map.retain(|_, v| v.hash == claim.hash);

                            if abandoned_claim_map.len() as f64
                                / (miner.claim_map.len() as f64 - 1.0)
                                > VALIDATOR_THRESHOLD
                            {
                                miner.claim_map.retain(|_, v| v.hash != claim.hash);
                            }
                            if let Err(_) =
                                app_sender.send(Command::UpdateAppMiner(miner.as_bytes()))
                            {
                                info!("Error sending updated miner to app")
                            }
                            miner.mining = true;
                            if let Err(e) = miner_sender.send(Command::MineBlock) {
                                info!("Error sending miner sender MineBlock: {:?}", e);
                            }
                        }
                    },
                    _ => {},
                }
            }
        }
    });
}
*/
