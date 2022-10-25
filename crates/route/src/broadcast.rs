
use futures::channel::mpsc;
use futures_timer::Delay;

use tokio::net::UdpSocket;
use tokio::{io, task};

use log::{error, info};

use std::net::{Ipv4Addr, Ipv6Addr, IpAddr};

use std::str::FromStr;
use std::time::Duration;

use std::{
    result::Result as StdResult,
};

use crate::error::DiscovererError;

pub type Result<T> = StdResult<T, DiscovererError>;
use crate::message::NodeRouteEntry;

const DISCOVERY_DELAY_SECS: u64 = 3;

pub fn node_discoverer_start(
        node_route_message: NodeRouteEntry,
        local_bind_address: String,        // "0.0.0.0" or specific interface 192.168.1.10
        bind_sender_local_address: String, // "0.0.0.0" or specific interface 192.168.1.10
        broadcast_target_address: String,  // "255.255.255.255" or specific sub network 192.168.255.255
        discovery_port: u16,               // common discovery port
    ) -> mpsc::UnboundedReceiver<NodeRouteEntry> {
        
    info!("Discoverer started on {}", &local_bind_address.as_str());

    let node_id = node_route_message.id();
    let (tx, rx) = mpsc::unbounded();

    let listen = 
        listen_on(
            local_bind_address.clone(),
            discovery_port,
            node_id,
            tx);

    let broadcast = 
        broadcast_network_discovery(
            bind_sender_local_address.clone(),
            broadcast_target_address,
            discovery_port,
            node_route_message,
        );

    task::spawn(async {
        listen 
    });

    task::spawn(async {
        broadcast 
    });

    rx
}

async fn broadcast_network_discovery(
        bind_sender_local_address: String, // "0.0.0.0" or specific interface 192.168.1.10
        broadcast_target_address: String,  // "255.255.255.255" or specific sub network 192.168.255.255
        discovery_port: u16,               // common discovery port
        node_route_message: NodeRouteEntry
    ) -> io::Result<()> {
    
        match Ipv4Addr::from_str(broadcast_target_address.as_str()) {
        
                Ok(ipv4_broadcast) => {
            
                let udp_sender_socket =
                    UdpSocket::bind(
                        format!("{}:{}", bind_sender_local_address, 0)
                    ).await?;

                udp_sender_socket.set_broadcast(true)?;

                let udp_target_broadcast = (ipv4_broadcast, discovery_port);

                let discovery_node_route_serialized_message = node_route_message.to_bytes();
        
                loop {
                    udp_sender_socket
                        .send_to(
                            &discovery_node_route_serialized_message,
                            udp_target_broadcast
                        )
                        .await?;
            
                    Delay::new(Duration::from_secs(DISCOVERY_DELAY_SECS)).await?;
                }
            }

            Err(e) => {
                error!("Cannot parse the specified address {} : {}", &broadcast_target_address.clone(), e);
                Ok(())
            }
        }

}

async fn listen_on(
    bind_sender_local_address: String,
    discovery_port: u16,
    node_id: String,
    tx: mpsc::UnboundedSender<NodeRouteEntry>,
) -> io::Result<()> {

    match Ipv4Addr::from_str(bind_sender_local_address.as_str()) {
        
        Ok(ipv4_local) => {

            let listen_address = (ipv4_local, discovery_port);
            let udp_socket = UdpSocket::bind(listen_address).await?;
        
            let mut buf = vec![0u8; 65535];
        
            loop {
        
                let (bytes_read, from_node) = udp_socket.recv_from(&mut buf).await?;
        
                log::debug!("Received {} bytes on the {} from Node : {}", bytes_read, &bind_sender_local_address, from_node.ip());
                
                let mut node_route_message = match NodeRouteEntry::from_bytes(&buf[0..bytes_read]) {
                    
                    Ok(node_route_message) => node_route_message,
                    
                    Err(e) => {
                        error!("Invalid discovery message received: {:#?}, from: {}", e, from_node);
                        continue;
                    }
                };

                match from_node.ip() {
                    IpAddr::V4(ipv4) => {
                        node_route_message.node_address_v4.clone_from(&ipv4);
                    }
                    IpAddr::V6(ipv6) => {
                        node_route_message.node_address_v6.clone_from(&ipv6);                        
                    }
                }

                if node_route_message.id() == node_id {
                    // if that's the same node ...
                    continue;
                }
        
                if tx.unbounded_send(node_route_message).is_err() {
                    // Sending error
                    break;
                }
            }
        
            Ok(())        
        }
        Err(e) => {
            error!("Cannot parse the specified address {} : {}", &bind_sender_local_address.clone(), e);
            Ok(())
        }
    }

}
