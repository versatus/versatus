use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct StopSignal;

/// The duration of time, in seconds, potential peers must wait in order to
/// register in the [Event::GeneratePayloadForPeerRegistration]. The interval
/// at which register requests can be made.
pub const REGISTER_REQUEST: Duration = Duration::new(60, 0);
/// The duration of time, in seconds, potential peers must wait before they can
/// be validated in the [Event::InitiateSyncPeers].
pub const RETRIEVE_PEERS_REQUEST: Duration = Duration::new(30, 0);
/// The duration of time, in seconds, before qualified peer namespaces can be
/// pulled from the rendezvous nodes in the [Event::PullFarmerNamespaces].
pub const SYNC_FARMER_NAMESPACES_REQUEST: Duration = Duration::new(60, 0);
