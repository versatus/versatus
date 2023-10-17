/// A custom trait that can be implemented on any type that can be owned by a
/// wallet i.e. Token, Claim, (Program?)
pub trait Ownable {
    type Pubkey;
    type SocketAddr;
    fn get_public_key(&self) -> Self::Pubkey;
    fn get_socket_addr(&self) -> Self::SocketAddr;
}
