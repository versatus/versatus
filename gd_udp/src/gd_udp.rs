use events::events::{write_to_json, VrrbNetworkEvent};
use log::info;
use messages::message::AsMessage;
use messages::message_types::MessageType;
use messages::packet::{Packet, Packetize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub struct GDUdp {
    pub sock: UdpSocket,
    pub addr: String,
    pub buf: Vec<u8>,
    pub buf_cursor: usize,
    pub inbox: HashMap<String, HashMap<u32, Packet>>,
    pub outbox: HashMap<String, HashMap<u32, (HashSet<SocketAddr>, HashSet<SocketAddr>, Packet)>>,
    pub to_node_sender: UnboundedSender<Vec<u8>>,
    pub to_inbox_receiver: UnboundedReceiver<(Packet, SocketAddr)>,
    pub timer: Instant,
    pub log: String,
}

impl GDUdp {
    pub const MAINTENANCE: Duration = Duration::from_millis(250);
    pub const RETURN_RECEIPT: u8 = 1u8;
    pub const NO_RETURN_RECIEPT: u8 = 0u8;

    pub fn maintain(&mut self) {
        self.outbox.iter_mut().for_each(|(_, map)| {
            map.retain(|_, (sent_set, ack_set, _)| sent_set != ack_set);
        });

        self.outbox.retain(|_, map| !map.is_empty());

        self.outbox.iter().for_each(|(_, map)| {
            map.iter().for_each(|(_, (sent_set, ack_set, packet))| {
                let resend: HashSet<_> = sent_set.difference(ack_set).collect();
                resend.iter().for_each(|peer| {
                    self.sock
                        .send_to(&packet.as_bytes(), peer)
                        .expect("Error sending packet to peer");
                });
            });
        });
        self.log_outbox().expect("Unable to log outbox");
    }

    pub fn log_inbox(&self) -> Result<(), serde_json::Error> {
        let event = VrrbNetworkEvent::VrrbInboxUpdate {
            inbox: self.inbox.clone(),
        };
        write_to_json(self.log.clone(), event)
    }

    pub fn log_outbox(&self) -> Result<(), serde_json::Error> {
        let event = VrrbNetworkEvent::VrrbOutboxUpdate {
            outbox: self.outbox.clone(),
        };
        write_to_json(self.log.clone(), event)
    }

    pub fn log_ack(&self, message: MessageType) -> Result<(), serde_json::Error> {
        let event = VrrbNetworkEvent::VrrbAckSent { message };
        write_to_json(self.log.clone(), event)
    }

    pub fn check_time_elapsed(&mut self) {
        let now = Instant::now();
        let time_elapsed = now.duration_since(self.timer);

        if time_elapsed >= GDUdp::MAINTENANCE {
            info!("Time to maintain outbox");
            self.maintain();
            self.timer = Instant::now();
        }
    }

    pub fn process_ack(&mut self, id: String, packet_number: u32, src: String) {
        let acker_addr: SocketAddr = src.parse().expect("Unable to parse socket address");
        info!("Received ack messages, processing...");
        if let Some(map) = self.outbox.get_mut(&id) {
            info!("Found message that was acked in outbox");
            if let Some((_, ack_set, _)) = map.get_mut(&packet_number) {
                info!("Found packet that was acked in ack_map");
                ack_set.insert(acker_addr);
                info!("Inserted the acker's addr into the ack set: {:?}", ack_set);
            }
        }
        self.log_outbox().expect("Unable to log outbox");
    }

    pub fn recv_to_inbox(&mut self) {
        if let Ok((packet, src)) = self.to_inbox_receiver.try_recv() {
            let id = String::from_utf8_lossy(&packet.id).to_string();
            let packet_number = usize::from_be_bytes(packet.clone().convert_packet_number()) as u32;
            if let Some(map) = self.inbox.get_mut(&id) {
                map.insert(packet_number, packet.clone());
            } else {
                let mut packets = HashMap::new();
                packets.insert(packet_number, packet.clone());
                self.inbox.insert(id.clone(), packets);
            }

            self.log_inbox().expect("Unable to log inbox");

            if packet.return_receipt == 1u8 {
                let message = MessageType::AckMessage {
                    packet_id: id,
                    packet_number,
                    src: self.addr.clone(),
                };

                self.ack(&src, message.clone());

                self.log_ack(message.clone()).expect("Unable to log ack");
            }

            let inbox = serde_json::to_string(&self.inbox)
                .unwrap()
                .as_bytes()
                .to_vec();

            if let Err(e) = self.to_node_sender.send(inbox) {
                info!("error sending packet to node for processing: {:?}", e);
            }

            self.inbox.retain(|_, packet_map| {
                packet_map.retain(|_, packet| {
                    let total_packets = usize::from_be_bytes(packet.clone().convert_total_packets());
                    total_packets != 1
                });
                !packet_map.is_empty()
            });
        }
    }

    pub fn send_reliable(&mut self, peer: &SocketAddr, packet: Packet) {
        self.sock
            .send_to(&packet.as_bytes(), peer)
            .expect("Error sending packet to peer");
        info!("Sent packet to peer {:?}", peer);
        let packet_id = String::from_utf8_lossy(&packet.clone().id).to_string();
        let packet_number = usize::from_be_bytes(packet.clone().convert_packet_number()) as u32;
        if let Some(map) = self.outbox.get_mut(&packet_id) {
            if let Some((sent_set, _, _)) = map.get_mut(&packet_number) {
                sent_set.insert(peer.clone());
            } else {
                let mut sent_set = HashSet::new();
                let ack_set: HashSet<SocketAddr> = HashSet::new();
                sent_set.insert(peer.clone());
                map.insert(packet_number, (sent_set, ack_set, packet.clone()));
            }
        } else {
            let mut map = HashMap::new();
            let mut sent_set = HashSet::new();
            let ack_set: HashSet<SocketAddr> = HashSet::new();
            sent_set.insert(peer.clone());
            map.insert(packet_number, (sent_set, ack_set, packet.clone()));
            self.outbox.insert(packet_id, map);
        }

        self.log_outbox().expect("Unable to log outbox");
    }

    pub fn ack<M: AsMessage>(&mut self, peer: &SocketAddr, message: M) {
        let packets = message.into_message(0).as_packet_bytes();
        packets.iter().for_each(|packet| {
            self.sock
                .send_to(packet, peer)
                .expect("Unable to send message to peer");
        });
    }
}
