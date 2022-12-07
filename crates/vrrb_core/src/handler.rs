/// This module is the primary allocator in the system, it contains the data
/// structures and the methods required to send commands to different parts of
/// the system.

/// A Basic trait to be implemented on any type of handler so that they can have
/// the basic allocation function
//TODO: Discuss if we ant to replace some of this stuff with DAG for more
// efficient command allocation.
// TODO #2 (12/09/22): Consider deprecating this altotgether
pub trait Handler<T, V> {
    fn send(&self, message: T) -> Option<T>;
    fn recv(&mut self) -> Option<V>;
}
