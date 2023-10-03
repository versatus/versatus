use hbbft::{
    crypto::{PublicKey, PublicKeySet},
    sync_key_gen::{Ack, Part},
};
use primitives::NodeId;
use vrrb_config::ThresholdConfig;

use crate::result::Result;

pub type SenderId = NodeId;
pub type ReceiverId = NodeId;

/// This is a trait that is implemented by the `DkgEngine` struct. It contains
/// the functions that are required to run the DKG protocol.
pub trait DkgGenerator {
    fn generate_partial_commitment(&mut self, threshold: usize) -> Result<(Part, NodeId)>;

    fn ack_partial_commitment(
        &mut self,
        sender_node_id: SenderId,
    ) -> Result<(ReceiverId, SenderId, Ack)>;

    fn handle_ack_messages(&mut self) -> Result<()>; //Handle all ACK Messages from all other k-1 MasterNodes

    fn generate_key_sets(&mut self) -> Result<Option<PublicKeySet>>;

    fn add_peer_public_key(&mut self, node_id: NodeId, public_key: PublicKey);

    fn threshold_config(&self) -> ThresholdConfig;
}
