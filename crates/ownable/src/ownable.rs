/// A custom trait that can be implemented on any type that can be owned by a
/// wallet i.e. Token, Claim, (Program?)
pub trait Ownable {
    fn get_pubkey(&self) -> String;
}
