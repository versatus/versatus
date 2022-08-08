/// This module produces event logging and can largely be replaced or eliminated.
/// A protocol for logging events can be developed elsewhere. 
use serde::{Deserialize, Serialize};
use std::fs;
use log::info;
use std::fmt::Debug;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum VrrbNetworkEvent {
    VrrbStarted,
    VrrbProtocolEvent { event: String },
}

pub fn write_to_json<T: Debug>(path: String, event: &T) -> Result<(), serde_json::Error> {
    let content = fs::read_to_string(path.clone());
    if let Ok(string) = content {
        let result: Result<Vec<VrrbNetworkEvent>, serde_json::Error> =
            serde_json::from_str(&string);
        if let Ok(mut events) = result {
            let new_event = get_event(event);
            events.push(new_event);
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
            let new_event = get_event(event);
            let events = vec![new_event];
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

pub fn get_event<T: Debug>(event: &T) -> VrrbNetworkEvent {
    let event_string = format!("{:?}", event);
    VrrbNetworkEvent::VrrbProtocolEvent { event: event_string }
}