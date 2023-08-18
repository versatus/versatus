use hbbft::sync_key_gen::Part;
use primitives::NodeId;
use vrrb_config::ThresholdConfig;

use crate::{engine::SenderId, result::Result};

/// This is a trait that is implemented by the `DkgEngine` struct. It contains
/// the functions that are required to run the DKG protocol.
pub trait DkgGenerator {
    fn generate_partial_commitment(&mut self, threshold: usize) -> Result<(Part, NodeId)>;

    fn ack_partial_commitment(&mut self, sender_node_id: SenderId) -> Result<()>;

    fn handle_ack_messages(&mut self) -> Result<()>; //Handle all ACK Messages from all other k-1 MasterNodes

    fn generate_key_sets(&mut self) -> Result<()>;

    fn threshold_config(&self) -> ThresholdConfig;
}
