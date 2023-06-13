use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct StopSignal;

/// The duration of time, in seconds, a potential peer must wait in order to
/// register in the [Event::GeneratePayloadForPeerRegistration]. The interval
/// at which peers can register to join a neighboring quorum pool.
pub const REGISTER_REQUEST: u64 = Duration::new(60, 0).as_secs();
/// The duration of time, in seconds, a potential peer must wait before they can
/// be validated in the [Event::InitiateSyncPeers].
/// The interval at which potential peers can be evaluated as they attempt to
/// join the neighboring quorum pool.
pub const RETRIEVE_PEERS_REQUEST: u64 = Duration::new(30, 0).as_secs();
/// The duration of time, in seconds, before a qualified peer can be added
/// to the neighboring quorum pool in the [Event::UpdateFarmerNamespaces].
pub const SYNC_FARMER_NAMESPACES_REQUEST: u64 = Duration::new(60, 0).as_secs();
