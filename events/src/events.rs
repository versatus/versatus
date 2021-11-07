use log::info;
use messages::packet::Packet;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::fs;
use std::net::SocketAddr;
use messages::message_types::MessageType;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum VrrbNetworkEvent {
    VrrbStarted,
    VrrbInboxUpdate {
        inbox: HashMap<String, HashMap<u32, Packet>>,
    },
    VrrbOutboxUpdate {
        outbox: HashMap<String, HashMap<u32, (HashSet<SocketAddr>, HashSet<SocketAddr>, Packet)>>,
    },
    VrrbAckReceived {
        packet: String,
        src: String,
    },
    VrrbAckSent {
        message: MessageType,
    }
}

pub fn write_to_json(
    path: String,
    event: VrrbNetworkEvent,
) -> Result<(), serde_json::Error> {
    let content = fs::read_to_string(path.clone());
    if let Ok(string) = content {
        let result: Result<Vec<VrrbNetworkEvent>, serde_json::Error> =
            serde_json::from_str(&string);
        if let Ok(mut events) = result {
            events.push(event);
            if events.len() > 100 {
                events.remove(0);
            }
            let json_vec = serde_json::to_vec(&events);
            if let Ok(json) = json_vec {
                if let Err(e) = fs::write(path.clone(), json) {
                    info!("Error writing event to events.json: {:?}", e);
                }
            }
        } else {
            let events = vec![event];
            let json_vec = serde_json::to_vec(&events);
            if let Ok(json) = json_vec {
                if let Err(e) = fs::write(path.clone(), json) {
                    info!("Error writing event to events.json: {:?}", e);
                }
            }
        }
    }
    Ok(())
}
