use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

use chrono::Utc;
use futures::{future, StreamExt};
use primitives::NodeType;
use queues::{CircularBuffer, IsQueue};
use telemetry::{debug, error, info};

use crate::{
    broker::{broker_retrieve, broker_server_start},
    context::{ContextHandler, NodeUpdateState},
    discovery::{node_discoverer_start, route_table_cleaning_routine_start},
    message::NodeRouteEntry,
};

pub const MESSAGE_CACHE_SIZE: usize = 100;

/// Local context containing current state of the local node boostrap syncing.
pub struct BootstrapContext {
    node_ip_addr: IpAddr,         // Node address
    node_distance_in_millis: i64, // Node distance in milliseconds
    is_local_node_boostrapped: bool, /* Is local Node already boostrapped and ready to serve
                                   * contents of its localstate. */
}

impl BootstrapContext {
    fn init(is_node_already_boostrapped: bool) -> Self {
        Self {
            node_ip_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
            node_distance_in_millis: 1000,
            is_local_node_boostrapped: is_node_already_boostrapped,
        }
    }
}

/// Node syncing context start
///
/// # Arguments
/// * `offset_localstate_file` - default file offset in localstate file
//  TODO: to be integrated into Block & chain.
pub async fn node_bootstrap_syncing_context_start(offset_localstate_file: u64) {
    let context = ContextHandler::init();

    let mut node_route_message = NodeRouteEntry::new(
        context.get().borrow().node_id.to_string(),
        NodeType::Archive,
        context.get().borrow().discovery_port,
    );

    let node_type = context.get().borrow().node_type;

    let mut bootstrap_context =
        BootstrapContext::init(context.get().borrow().node_state == NodeUpdateState::UpToDate);

    if bootstrap_context.is_local_node_boostrapped {
        node_route_message.node_state = NodeUpdateState::UpToDate;

        broker_server_start(context.clone(), offset_localstate_file);
    }

    let received_messages = node_discoverer_start(context.clone(), node_route_message);

    info!(
        "Start of {}, on address {}",
        context.get().borrow().node_id,
        context.get().borrow().bind_sender_local_address.clone()
    );

    let mut cache = CircularBuffer::<Arc<NodeRouteEntry>>::new(MESSAGE_CACHE_SIZE);

    route_table_cleaning_routine_start(context.clone());

    received_messages
        .for_each(move |m| {
            debug!("Received: {:?}", m);

            let now = Utc::now().timestamp_millis();
            let node_distance_in_millis = now - m.time;

            context
                .get()
                .borrow_mut()
                .node_routes_db
                .add_node(
                    m.node_id.clone(),
                    if node_distance_in_millis > 0 {
                        node_distance_in_millis
                    } else {
                        0
                    },
                    m.node_type,
                    m.ip(),
                )
                .add_addrs_v4(m.node_id.clone(), m.ip_v4s())
                .add_addrs_v6(m.node_id.clone(), m.ip_v6s());

            if !bootstrap_context.is_local_node_boostrapped
                && m.node_state == NodeUpdateState::UpToDate
                && bootstrap_context.node_distance_in_millis >= node_distance_in_millis
                && m.node_type == node_type
            {
                bootstrap_context.node_distance_in_millis = node_distance_in_millis;
                bootstrap_context.node_ip_addr = m.ip();

                match broker_retrieve(context.clone(), m.ip()) {
                    Ok(retrieved_bytes) => {
                        info!(
                            "Node was correctly bootstrapped with {} bytes.",
                            retrieved_bytes
                        );

                        bootstrap_context.is_local_node_boostrapped = true;
                        context
                            .get()
                            .borrow_mut()
                            .set_state(NodeUpdateState::UpToDate);

                        let offset_localstate_in_file = 0;

                        info!("Start serving data for other Nodes.");
                        broker_server_start(context.clone(), offset_localstate_in_file);
                    },
                    Err(e) => {
                        bootstrap_context.is_local_node_boostrapped = false;
                        context
                            .get()
                            .borrow_mut()
                            .set_state(NodeUpdateState::Invalid);

                        error!("Error : {:#?}", e);
                    },
                }

                info!("Node was correctly boostrapped, ready for data service and processing...");
            }

            let m_arc = Arc::new(m);
            if let Err(e) = cache.add(m_arc) {
                error!("Error adding message to a cache {:#?}", e)
            }

            future::ready(())
        })
        .await
}
