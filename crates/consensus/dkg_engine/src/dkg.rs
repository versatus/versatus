use std::sync::Arc;

use hbbft::sync_key_gen::{PartOutcome, SyncKeyGen};
use primitives::{NodeId, NodeType};
use rand::rngs::OsRng;

use crate::result::{DkgError, DkgResult};

/// This is a trait that is implemented by the `DkgEngine` struct. It contains
/// the functions that are required to run the DKG protocol.
pub trait DkgGenerator {
    type DkgStatus;
    // type Return;

    fn generate_sync_keygen_instance(&mut self, threshold: usize) -> Self::DkgStatus;

    // PartOutCome to be sent to channel for broadcasting it to other peers
    fn ack_partial_commitment(&mut self, node_id: NodeId) -> Self::DkgStatus;

    // Handle all ACK Messages from all other k-1 MasterNodes
    fn handle_ack_messages(&mut self) -> Self::DkgStatus;

    fn generate_key_sets(&mut self) -> Self::DkgStatus;
}
