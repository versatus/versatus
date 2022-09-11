/// A custom trait that can be implemented on a custom type that needs to retain
/// a nonce, i.e. Tx, Claim, Token, (Program?), etc.
pub trait Nonceable {
    fn nonceable(&self) -> bool;
    fn nonce_up(&mut self);
}
