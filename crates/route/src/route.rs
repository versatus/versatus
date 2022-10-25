
use futures::{future, StreamExt};

use log::error;
use routerswarmcast::{
    broadcast::{node_discoverer_start},
    message::{NodeRouteEntry, NodeType},
    context
};

use routerswarm::network;                                                                                                                                                                     
use routerswarm::context;                                                                                                                                                                     
                                                                                                                                                                                              
use tokio::time::{sleep, Duration};

use uuid::Uuid;

pub async fn route_discoverer() {                                                                                                                                                             
                                                                                                                                                                                              
    let mut context = context::AppContext::new();                                                                                                                                             
                                                                                                                                                                                              
    let node_id = Uuid::new_v4();

    // TODO: to be moved to common config !
    let local_bind_address = String::from("0.0.0.0");
    let broadcast_target_address = String::from("255.255.255.255");
    let UDP_DISCOVERY_PORT: u16 = 5330;

    let node_route_message = NodeRouteEntry::new(node_id.to_string(), NodeType::Archive, UDP_DISCOVERY_PORT);
    
    let received_messages = 
                node_discoverer_start(
                    node_route_message,
                    local_bind_address.clone(), 
                    local_bind_address.clone(),
                    broadcast_target_address,
                    UDP_DISCOVERY_PORT
                );    
   
    tokio::spawn(async move {
        log::info!("Start of {}, on address {}", node_id, local_bind_address.clone());
        received_messages.for_each(move |m| {
            log::debug!("Received: {:?}", m);
            match context.node_routes_db.add_node(&m.node_id, m.ip()) {
                Ok(_) => {}
                Err(e) => {
                    error!("Error update current node {:#?}", e)
                }
            }
            future::ready(())
        }).await;
        sleep(Duration::from_millis(100)).await;
    });

}   
