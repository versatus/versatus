use messages::packet::Packet;
use std::io;
use std::net::{SocketAddr, UdpSocket};

pub fn recv_msg(sock: &UdpSocket) -> io::Result<(usize, SocketAddr, Packet)> {
    let mut buf = [0; 65535];
    match sock.recv_from(&mut buf) {
        Err(e) => {
            return Err(e);
        }
        Ok((amt, src)) => {
            let packet = Packet::from_bytes(&buf[..amt]);
            return Ok((amt, src, packet))
        },
    }
}
