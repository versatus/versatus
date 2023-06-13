use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct StopSignal;

/// The duration of time, in seconds, a potential peer must wait in order to
/// register in the [Event::GeneratePayloadForPeerRegistration]. The interval
/// at which peers can register to join a neighboring quorum pool.
pub const REGISTER_REQUEST: Duration = Duration::new(60, 0);
/// The duration of time, in seconds, a potential peer must wait before they can
/// be validated in the [Event::InitiateSyncPeers].
/// The interval at which potential peers can be evaluated as they attempt to
/// join the neighboring quorum pool.
pub const RETRIEVE_PEERS_REQUEST: Duration = Duration::new(30, 0);
/// The duration of time, in seconds, before qualified peer namespaces can be
/// pulled from the neighboring quorum pool in the
/// [Event::PullFarmerNamespaces].
pub const SYNC_FARMER_NAMESPACES_REQUEST: Duration = Duration::new(60, 0);
