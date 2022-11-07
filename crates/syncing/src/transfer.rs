use std::net::{SocketAddr, IpAddr, AddrParseError};
use log::{error, debug};
use udt::{UdtSocket, UdtError, UdtOpts, SocketType, SocketFamily};

use crate::error::DataBrokerError;

const BUFFER_SIZE: i32 = 4096000;

fn init_udt_socket() -> UdtSocket {
    udt::init();
    let udt_socket = match UdtSocket::new(SocketFamily::AFInet, SocketType::Stream) {
        Ok(sock) => sock,
        Err(e) => {
            error!("init_udt_socket: Udt creation error : {:#?}", e);
            todo!();
        }
    };
    if let Err(e) = udt_socket.setsockopt(UdtOpts::UDP_RCVBUF, BUFFER_SIZE) {
        error!("init_udt_socket: Setting option UDP_RCVBUF: {:#?}", e);
    }
    if let Err(e) = udt_socket.setsockopt(UdtOpts::UDP_SNDBUF, BUFFER_SIZE) {
        error!("init_udt_socket: Setting option UDP_SNDBUF: {:#?}", e);
    }
    udt_socket
}

fn init_socket_addr(server_ip: String, server_port: u16) -> Result<SocketAddr, AddrParseError> {
    let connection_string = format!("{}:{}", server_ip, server_port);
    match connection_string.parse::<SocketAddr>() {
        Ok(socket) => Ok(socket),
        Err(e) => {
            error!("Client connection error to {} : {}", connection_string, e);
            Err(e)
        }
    }
}

trait UDTransfer {
    fn send_udt_data(&self, buf: &[u8]) -> Result<usize, UdtError>;
    fn recv_udt_data(&self, buf: &mut Box<Vec<u8>>) -> Result<usize, UdtError>;
}

impl UDTransfer for UdtSocket {

    fn send_udt_data(&self, buf: &[u8]) -> Result<usize, UdtError> {
        let mut total_size: usize = 0;
        let buf_size: [u8; 8] = buf.len().to_be_bytes();
        match self.send(&buf_size) {
            Ok(8) => {
                log::debug!("Received correct transfer format : {} Bytes to be transferred.", buf.len());
                while total_size < buf.len() {
                    match self.send(&buf[total_size..]) {
                        Ok(delta) => {
                            total_size +=  delta as usize;
                            debug!("Sent {} Bytes and {} Bytes total.", delta, total_size);
                        },
                        Err(e) => {
                            log::error!("UDT Data sending error : {:#?}", e);
                            break;
                        }
                    }
                }
                log::debug!("Transfer complete, {} Bytes was transferred.", total_size);
                Ok(total_size)
            }
            Ok(size) =>  {
                let msg = format!("UDT protocol error, wrong size : {}", size);
                log::error!("{}", msg);
                Err(UdtError{err_code: 10000, err_msg: msg})
            }
            Err(e) => {
                log::error!("UDT Data sending error : {:#?}", e);
                Err(e)
            }
        }
    }

    fn recv_udt_data(&self, buf: &mut Box<Vec<u8>> /*[u8]*/) -> Result<usize, UdtError> {
        let mut total_size: usize = 0;
        let mut buf_size: [u8; 8] = [0u8; 8];
        match self.recv(&mut buf_size, 8) {
            Ok(8) => {
                let len = usize::from_be_bytes(buf_size);
                log::debug!("Received correct transfer format : {} Bytes to be received.", len);
                if buf.len() < len  {
                    buf.resize(len, 0u8);
                }
                while total_size < len {
                    let remaining = len - total_size;
                    match self.recv(&mut buf[total_size..], remaining) {
                        Ok(delta) => {
                            total_size += delta as usize;
                            debug!("Received {} Bytes and {} Bytes total.", delta, total_size);
                        },
                        Err(e) => {
                            log::error!("UDT Data receiving error : {:#?}", e);
                            break;
                        }
                    }
                }
                log::debug!("Transfer complete, {} Bytes was received.", total_size);
                Ok(total_size)        
            }
            Ok(size) =>  {
                let msg = format!("UDT protocol error, wrong size : {}", size);
                log::error!("{}", msg);
                Err(UdtError{err_code: 10000, err_msg: msg})
            }
            Err(e) => {
                log::error!("UDT Data receiving error : {:#?}", e);
                Err(e)
            }
        }
    }

}

#[derive(Debug)]
pub struct DATAServer {
    pub ip_addr: IpAddr,
    pub port: u16,
    sock: UdtSocket,
}

pub struct DATAClient {
    addr: SocketAddr,
    sock: UdtSocket,
}

pub struct DATAServerConnection {
    sock: UdtSocket,
}

pub struct DATAClientConnection {
    sock: UdtSocket,
}

impl DATAClient {
    pub fn _new(remote_ip: String, remote_port: u16) -> Result<DATAClient, DataBrokerError> {
        let sock = init_udt_socket();
        match init_socket_addr(remote_ip, remote_port) {
            Ok(addr) => Ok(DATAClient {
                                        addr: addr,
                                        sock: sock,
                                    }),
            Err(_) => Err(DataBrokerError::ConnectionError)
        }
    }

    pub fn new_from_ip_addr(remote_ip: IpAddr, remote_port: u16) -> DATAClient {
        let sock = init_udt_socket();
        let connection_string = format!("{}:{}", remote_ip.to_string(), remote_port);
        let addr = match connection_string.parse::<SocketAddr>() {
            Ok(addr) => addr,
            Err(e) => {
                error!("IpAddr parse error : {:#?}", e);
                format!("127.0.0.1:0").parse::<SocketAddr>().unwrap()
            }
        };

        DATAClient {
            addr: addr,
            sock: sock,
        }
    }
    
    pub fn connect(&self) -> Result<DATAClientConnection, UdtError> {
        debug!("Connecting to : {:#?} ...", self.addr);
        match self.sock.connect(self.addr) {
            Ok(_) => 
                Ok(DATAClientConnection {
                    sock: self.sock,
                }),
            Err(e) => Err(e)
        }
    }

    pub fn get_ip(&self) -> &SocketAddr {
        &self.addr
    }
}

impl DATAServer {
    
    pub fn new(ip_addr: IpAddr, port: u16) -> DATAServer {
        let sock = init_udt_socket();
        let sock_addr = SocketAddr::new(ip_addr, port);
        if let Err(e) = sock.bind(sock_addr) {
            error!("DATAServer::new, binding error : {:#?}", e);
        }
        DATAServer {
            sock: sock,
            ip_addr: ip_addr,
            port: port,
        }
    }
    
    pub fn listen(&self) -> Result<(), UdtError> {
        self.sock.listen(1)
    }
    
    pub fn accept(&self) -> Result<DATAServerConnection, UdtError> {
        self.sock.accept().map(move |(sock, _)| {
            DATAServerConnection {
                sock: sock,
            }
        })
    }
}

impl DATAServerConnection {
    pub fn get_name(&self) -> Result<SocketAddr, UdtError> {
        self.sock.getpeername()
    }

    pub fn send(&self, buffer: &[u8]) -> Result<usize, UdtError> {
        self.sock.send_udt_data(buffer)
    }

    pub fn _recv(&self, buffer:  &mut Box<Vec<u8>>) -> Result<usize, UdtError> {
        self.sock.recv_udt_data(buffer)
    }
}

impl DATAClientConnection {
    pub fn _send(&self, buffer: &[u8]) -> Result<usize, UdtError> {
        self.sock.send_udt_data(buffer)
    }

    pub fn recv(&self, buffer:  &mut Box<Vec<u8>>) -> Result<usize, UdtError> {
        self.sock.recv_udt_data(buffer)
    }
}
