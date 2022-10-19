use libp2p::{
    core::upgrade,
    tcp::{GenTcpConfig, TokioTcpTransport},
    mdns::{Mdns, MdnsEvent},
    noise,
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mplex,
    futures::StreamExt,
    NetworkBehaviour, PeerId, Transport,
    noise::{Keypair, X25519Spec},
    swarm::{Swarm, SwarmBuilder, SwarmEvent}, Multiaddr,
    ping::{Ping, PingConfig, PingEvent, PingSuccess},
};

use log::{error};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    process,
    env,
    time::Duration,
    borrow::BorrowMut,
};

use crate::{lrnodepool::{NodeAddrMap, NodeAddr}, context::AppContext};

const TOPIC_NAME: &str = "Peers";
const TOR_ONION_ADDR: &str = "TOR_ONION_ADDR";

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static NODE_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new(TOPIC_NAME));

#[derive(Debug, Serialize, Deserialize)]
struct NodeListMessage {
    data: NodeAddrMap,
}

#[derive(Debug, Serialize, Deserialize)]
enum ListMode {
    DelOne(String),
    GetOne(String),
    GetAll,
}

#[derive(Debug, Serialize, Deserialize)]
struct NodeListRequest {
    mode: ListMode,
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "NetBehaviourEvent")]
struct NetBehaviour {
    // gossipsub: Gossipsub, just for testing
    floodsub: Floodsub,
    mdns: Mdns,
    ping: Ping,
}

#[derive(Debug)]
enum NetBehaviourEvent {
    Floodsub(FloodsubEvent),
    Mdns(MdnsEvent),
    Ping(PingEvent),
}

impl From<FloodsubEvent> for NetBehaviourEvent {
    fn from(event: FloodsubEvent) -> Self {
        NetBehaviourEvent::Floodsub(event)
    }
}

impl From<MdnsEvent> for NetBehaviourEvent {
    fn from(event: MdnsEvent) -> Self {
        NetBehaviourEvent::Mdns(event)
    }
}

impl From<PingEvent> for NetBehaviourEvent {
    fn from(event: PingEvent) -> Self {
        NetBehaviourEvent::Ping(event)
    }
}

pub async fn routing_discoverer_start(context: &mut AppContext) {

    log::info!("Random local Node peerId : {}", NODE_ID.clone());

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&KEYS)
        .expect("can create auth keys");
    
    let mut behaviour: NetBehaviour = NetBehaviour {
    
        floodsub: Floodsub::new(NODE_ID.clone()),
    
        mdns: Mdns::new(Default::default())
            .await
            .expect("can create mdns"),
    
        ping: Ping::new(
            PingConfig::new()
                .with_interval(Duration::from_secs(1))
                .with_keep_alive(true),
        ),
    };
    
    behaviour.floodsub.subscribe( TOPIC.clone( ) );
    
    let listen_on = match env::var_os(TOR_ONION_ADDR) {
        Some(tor_addr) => format!("/ip6/{}/tcp/0", tor_addr.into_string().unwrap()),
        None => context.args.full_bind_address.clone(),
    };
    
    let transport = TokioTcpTransport::new(GenTcpConfig::default().nodelay(true))
                                                        .upgrade(upgrade::Version::V1)
                                                        .authenticate(
                                                            noise::NoiseConfig::xx(auth_keys)
                                                                .into_authenticated()
                                                        )
                                                        .multiplex(mplex::MplexConfig::new())
                                                        .boxed();
    
    let mut swarm = SwarmBuilder::new(transport, behaviour, NODE_ID.clone())
                                            .executor(
                                                    Box::new(
                                                        |fut| {
                                                            tokio::spawn(fut);
                                                        }))
                                            .build();
    
    let parsed_full_address: Multiaddr = match listen_on.parse() {
        Ok(addr) => addr,
        Err(e) => {
            error!("\nerror parsing address [{}] : {}, Exiting...\n", listen_on, e);
            process::exit(0x1)
        },
    };
    
    match Swarm::listen_on(
        &mut swarm,
        parsed_full_address // typical value : "/ip4/0.0.0.0/tcp/0"
    ) {
        Ok(_) => log::info!("\nSwarm started on : {}", listen_on),
        Err(e) => error!("error starting Swarm on the address : {}, error = {}", listen_on, e)
    };
    
    routing_discoverer_event_loop(context.borrow_mut(), &mut swarm).await;

}

async fn routing_discoverer_event_loop( context: &mut AppContext,
                                        swarm: &mut Swarm<NetBehaviour>) {

    loop {
        tokio::select! {
            event = swarm.select_next_some() => {

                match event {

                    SwarmEvent::NewListenAddr { address, .. } => {
                        log::info!("Listening on {:?}", address);
                    }

                    SwarmEvent::Behaviour(
                        NetBehaviourEvent::Floodsub(
                            FloodsubEvent::Message(message))) => {

                                if let Ok(node_msg) = serde_json::from_slice::<NodeListMessage>(&message.data) {

                                    node_msg.data.iter().for_each(|n| log::debug!("{:?}", n));
                                    
                                    for (node, node_addr) in node_msg.data.iter() {

                                        if let Ok(bs58) = bs58::decode(node).into_vec() {
                                            
                                            if let Ok(peer) = PeerId::from_bytes(&bs58) {

                                                if ! swarm.behaviour().mdns.has_node( &peer ) {

                                                    if node_addr.addr_set.len() > 0 {
    
                                                        for addr in node_addr.addr_set.iter() {
    
                                                            _ = context.node_routes_db.add_node(&node, addr.clone());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                } else if let Ok(list_msg) = serde_json::from_slice::<NodeListRequest>(&message.data) {

                                    let response_json = match list_msg.mode {
                                        ListMode::GetAll => {
                                            let response: Vec<NodeAddr> = match context.node_routes_db.get_all_nodes() {
                                                Some(v) => v,
                                                None => Vec::new()
                                            };
                                            serde_json::to_string(&response).expect("can jsonify response")
                                        },
                                        ListMode::GetOne(addr) => {
                                            let mut response = Vec::new();
                                            if let Some(v) = context.node_routes_db.get_node(&addr) {
                                                response.push(v);
                                            }
                                            serde_json::to_string(&response).expect("can jsonify response")
                                        },
                                        ListMode::DelOne(addr) => {
                                            _ = context.node_routes_db.remove_node(&addr);
                                            let response: Vec<NodeAddr> = match context.node_routes_db.get_all_nodes() {
                                                Some(v) => v,
                                                None => Vec::new()
                                            };
                                            serde_json::to_string(&response).expect("can jsonify response")
                                        }
                                    };

                                    swarm
                                        .behaviour_mut()
                                        .floodsub
                                        .publish(TOPIC.clone(), response_json.as_bytes());

                                    log::debug!("==> Response to channel : {:?} => {:?}", TOPIC.clone(), response_json);

                                } else {

                                    log::debug!(
                                        "Received: '{:?}' from {:?}",
                                        String::from_utf8_lossy(&message.data),
                                        message.source
                                    );
                                }
                    }

                    SwarmEvent::Behaviour(
                        NetBehaviourEvent::Mdns(event)) => {

                            match event {

                                MdnsEvent::Discovered(discovered_node_list) => {
                                    for (node, addr) in discovered_node_list {

                                        swarm
                                            .behaviour_mut()
                                            .floodsub
                                            .add_node_to_partial_view(node);

                                        let node_id_encoded = node.to_base58();

                                        _ = context.node_routes_db.add_node(&node_id_encoded, addr.clone());

                                        let req = NodeListRequest {
                                            mode: ListMode::GetAll,
                                        };
                            
                                        let json_list_req = serde_json::to_string(&req).expect("can jsonify request");
                            
                                        swarm
                                            .behaviour_mut()
                                            .floodsub
                                            .publish(TOPIC.clone(), json_list_req.as_bytes());

                                        log::info!("==> Discovered Node : {:?} => {:?}", node, addr);
                                    }
                                }

                                MdnsEvent::Expired(expired_node_list) => {

                                    for (node, addr) in expired_node_list {

                                        if ! swarm.behaviour().mdns.has_node( &node ) {

                                            swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&node);

                                            let node_id_encoded = node.to_base58();
                                            _ = context.node_routes_db.remove_node(&node_id_encoded);

                                            let req = NodeListRequest {
                                                mode: ListMode::GetAll,
                                            };
                                
                                            let json_list_req = serde_json::to_string(&req).expect("can jsonify request");
                                
                                            swarm
                                                .behaviour_mut()
                                                .floodsub
                                                .publish(TOPIC.clone(), json_list_req.as_bytes());    

                                            log::info!("==> Expired Node : {:?} => {:?}", node, addr);
                                        }
                                    }
                                }
                            }
                    }

                    SwarmEvent::Behaviour(
                        NetBehaviourEvent::Ping(PingEvent {
                            peer,
                            result: Ok(PingSuccess::Ping { rtt }),
                        })) => {
                            log::debug!("==> Ping to : {} => {}ms", peer, rtt.as_millis());
                    }
                    
                    SwarmEvent::Behaviour(
                        NetBehaviourEvent::Ping(PingEvent {
                            peer,
                            result: Ok(PingSuccess::Pong),
                        })) => {
                            log::debug!("==> Pong from : {}", peer);
                    }
                    
                    _ => {}
                }
            }                
        }
    }
}

