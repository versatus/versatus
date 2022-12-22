use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
    time::Duration,
};

use chrono::Utc;
use futures::channel::mpsc;
use futures_timer::Delay;
use primitives::types::node::NodeType;
use telemetry::{debug, error, info};
use tokio::net::UdpSocket;

use crate::{context::ContextHandler, message::NodeRouteEntry};

const DISCOVERY_DELAY_SECS: u64 = 3;
const CLEAN_ROUTINE_DELAY_SECS: u64 = 7;
const MAX_TIMEOUT_SECS: i64 = 20;

/// function clearning "routing table" in a separate thread.
/// Removes nodes which were not responsing for more than MAX_TIMEOUT_SECS
/// seconds.
///
/// # Arguments
/// * `context`             - application context with predefined parameters
pub fn route_table_cleaning_routine_start(context: ContextHandler<'static>) {
    let ctx = context.clone();

    tokio::spawn(async move {
        loop {
            if let Err(e) = Delay::new(Duration::from_secs(CLEAN_ROUTINE_DELAY_SECS)).await {
                error!("route_table_cleaning_routine_start: System error : {}", e);
            }

            ctx.get()
                .borrow_mut()
                .node_routes_db
                .remove_inactive_nodes(MAX_TIMEOUT_SECS);
        }
    });
}

/// function which spawns threads broadcasting and receiving UDP messages about
/// the state of the networks. This mechanism serves both for updating all the
/// nodes about current state of network and all the nodes AND removal inactive
/// nodes.
///
/// # Arguments
/// * `context`             - application context with predefined parameters
/// * `node_route_message`  - a template of the broadcasted UDP state with
///   predefined values.
pub fn node_discoverer_start(
    context: ContextHandler<'static>,
    node_route_message: NodeRouteEntry,
) -> mpsc::UnboundedReceiver<NodeRouteEntry> {
    info!(
        "node_discoverer_start: Discoverer started on {}",
        context.get().borrow().bind_sender_local_address.as_str()
    );

    let (tx, rx) = mpsc::unbounded();

    let ctx = context.clone();
    let bind_sender_local_address = ctx.get().borrow().bind_sender_local_address.clone();
    let broadcast_target_address = ctx.get().borrow().broadcast_target_address.clone();

    // spreads information about the current node's state and its closest discovered
    // neighours.
    tokio::spawn(async move {
        match Ipv4Addr::from_str(broadcast_target_address.as_str()) {
            Ok(ipv4_broadcast) => {
                if let Ok(udp_sender_socket) =
                    UdpSocket::bind(format!("{}:{}", bind_sender_local_address, 0)).await
                {
                    if let Err(e) = udp_sender_socket.set_broadcast(true) {
                        error!("node_discoverer_start: Broadcast setting error : {}", e);
                    }

                    let udp_target_broadcast = (ipv4_broadcast, ctx.get().borrow().discovery_port);

                    let mut message_template = node_route_message.clone();

                    loop {
                        message_template.time = Utc::now().timestamp_millis();
                        message_template.node_state = *ctx.get().borrow_mut().state();

                        if let Some(cluster) = ctx
                            .get()
                            .borrow_mut()
                            .node_routes_db
                            .get_cluster(NodeType::Full)
                        {
                            let mut cluster_ipv4s: Vec<Ipv4Addr> = cluster
                                .clone()
                                .into_iter()
                                .flat_map(|n| {
                                    Vec::from_iter(n.addr_set)
                                        .into_iter()
                                        .filter(|a| a.is_ipv4())
                                })
                                .map(|a| match a {
                                    IpAddr::V4(a) => a,
                                    IpAddr::V6(_) => Ipv4Addr::LOCALHOST,
                                })
                                .collect();

                            cluster_ipv4s.sort();
                            cluster_ipv4s.dedup();
                            message_template.cluster_ipv4 = cluster_ipv4s;

                            let mut cluster_ipv6s: Vec<Ipv6Addr> = cluster
                                .into_iter()
                                .flat_map(|n| {
                                    Vec::from_iter(n.addr_set)
                                        .into_iter()
                                        .filter(|a| a.is_ipv6())
                                })
                                .map(|a| match a {
                                    IpAddr::V4(_) => Ipv6Addr::LOCALHOST,
                                    IpAddr::V6(b) => b,
                                })
                                .collect();

                            cluster_ipv6s.sort();
                            cluster_ipv6s.dedup();
                            message_template.cluster_ipv6 = cluster_ipv6s;
                        }

                        let discovery_node_route_serialized_message = message_template.to_bytes();

                        if let Err(e) = udp_sender_socket
                            .send_to(
                                &discovery_node_route_serialized_message,
                                udp_target_broadcast,
                            )
                            .await
                        {
                            error!("node_discoverer_start: Sending error : {}", e);
                        }

                        if let Err(e) = Delay::new(Duration::from_secs(DISCOVERY_DELAY_SECS)).await
                        {
                            error!("node_discoverer_start: System error : {}", e);
                        }
                    }
                }
            },

            Err(e) => {
                error!(
                    "node_discoverer_start: Cannot parse the specified address {} : {}",
                    &ctx.get().borrow().broadcast_target_address.clone(),
                    e
                );
            },
        }
    });

    // retrieves information from the network about active nodes.
    tokio::spawn(async move {
        let ctx1 = context.clone();
        let discovery_port = ctx1.get().borrow().discovery_port;
        let bind_sender_local_address = ctx1.get().borrow().bind_sender_local_address.clone();
        let node_id = ctx1.get().borrow().node_id.clone();

        match Ipv4Addr::from_str(bind_sender_local_address.as_str()) {
            Ok(ipv4_local) => {
                let listen_address = (ipv4_local, discovery_port);
                if let Ok(udp_socket) = UdpSocket::bind(listen_address).await {
                    let mut buf = vec![0u8; 65535];

                    loop {
                        if let Ok((bytes_read, from_node)) = udp_socket.recv_from(&mut buf).await {
                            let ts = Utc::now().timestamp_millis();

                            debug!(
                                "Received {} bytes on the {} from Node : {}",
                                bytes_read,
                                bind_sender_local_address,
                                from_node.ip()
                            );

                            let mut node_route_message =
                                match NodeRouteEntry::from_bytes(&buf[0..bytes_read]) {
                                    Ok(node_route_message) => node_route_message,

                                    Err(e) => {
                                        error!(
                                            "Invalid discovery message received: {:#?}, from: {}",
                                            e, from_node
                                        );
                                        continue;
                                    },
                                };

                            // ignore own messages !
                            if node_route_message.id() == node_id {
                                continue;
                            }

                            match from_node.ip() {
                                IpAddr::V4(ipv4) => {
                                    node_route_message.node_address_v4.clone_from(&ipv4);
                                },
                                IpAddr::V6(ipv6) => {
                                    node_route_message.node_address_v6.clone_from(&ipv6);
                                },
                            }

                            debug!(
                                "Network distance in millis : {}",
                                (ts - node_route_message.time())
                            );

                            if tx.unbounded_send(node_route_message).is_err() {
                                // Sending error
                                break;
                            }
                        }
                    }
                }
            },
            Err(e) => {
                error!(
                    "Cannot parse the specified address {} : {}",
                    context.get().borrow().bind_sender_local_address.clone(),
                    e
                );
            },
        }
    });

    rx
}
