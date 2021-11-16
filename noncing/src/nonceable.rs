pub trait Nonceable {
    fn nonceable(&self) -> bool;
    fn nonce_up(&mut self);
}