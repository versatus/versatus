use std::net::{SocketAddr, UdpSocket};
use std::collections::{HashMap, HashSet};
use messages::packet::{Packet, Packetize};
use messages::message_types::MessageType;
use messages::message::AsMessage;
use tokio::sync::mpsc::{UnboundedSender};
use log::info;

#[derive(Debug)]
pub struct GDUdp {
    pub sock: UdpSocket,
    pub buf: Vec<u8>,
    pub buf_cursor: usize,
    pub inbox: HashMap<String, HashMap<u32, Packet>>,
    pub outbox: HashMap<String, HashMap<u32, (HashSet<SocketAddr>, HashSet<SocketAddr>, Packet)>>,
    pub to_node_sender: UnboundedSender<Vec<u8>>,
}

impl GDUdp {
    pub fn maintain(&mut self) {
        self.outbox.iter_mut().for_each(|(_, map)| {
            map.retain(|_, (sent_set, ack_set, _)| {
                sent_set != ack_set
            });
        });

        self.outbox.retain(|_, map| { !map.is_empty() });
        
        self.outbox.iter().for_each(|(_, map)| {
            map.iter().for_each(|(_, (sent_set, ack_set, packet))| {
                let resend: HashSet<_> = sent_set.difference(ack_set).collect();
                resend.iter().for_each(|peer| {
                    self.sock.send_to(&packet.as_bytes(), peer).expect("Error sending packet to peer");
                });
            });
        });
    }

    pub fn recv_to_inbox(&mut self) {
        let (amt, src) = self.sock.recv_from(&mut self.buf).expect("No data received");

        if amt > 0 {
            let packet = Packet::from_bytes(&self.buf[self.buf_cursor..self.buf_cursor+amt]);
            let id = String::from_utf8_lossy(&packet.id).to_string();
            let packet_number = usize::from_be_bytes(packet.clone().convert_packet_number()) as u32;
            if let Some(map) = self.inbox.get_mut(&id) {
                map.insert(packet_number, packet);
            }

            let message = MessageType::AckMessage {
                packet_id: id,
                packet_number,
                src: src.to_string()
            };

            self.ack(&src, message);

            let inbox = serde_json::to_string(&self.inbox).unwrap().as_bytes().to_vec();

            if let Err(e) = self.to_node_sender.send(inbox) {
                info!("error sending packet to node for processing: {:?}", e);
            }
        }
    }

    pub fn send_reliable(&mut self, peer: &SocketAddr, packet: Packet) {
        self.sock.send_to(&packet.as_bytes(), peer).expect("Error sending packet to peer");
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
    }

    pub fn ack<M: AsMessage>(&mut self, peer: &SocketAddr, message: M) {
        let packets = message.into_message().as_packet_bytes();
        packets.iter().for_each(|packet| {
            self.sock.send_to(packet, peer).expect("Unable to send message to peer");
        });
    }
}

impl Clone for GDUdp {
    fn clone(&self) -> GDUdp {
        let cloned_sock = self.sock.try_clone().expect("Unable to clone socket");
        GDUdp {
            sock: cloned_sock,
            buf: self.buf.clone(),
            buf_cursor: self.buf_cursor.clone(),
            inbox: self.inbox.clone(),
            outbox: self.outbox.clone(),
            to_node_sender: self.to_node_sender.clone(),
        }
    }   
}