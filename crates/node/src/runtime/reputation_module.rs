use std::{
    collections::HashSet,
    fmt::Debug,
    hash::Hash,
    ops::{Add, AddAssign, Div, DivAssign, Mul, Sub, SubAssign},
};

use async_trait::async_trait;
use buckets::bucketize::BucketizeSingle;
use decentrust::{
    cms::CountMinSketch,
    honest_peer::{HonestPeer, Update},
    probabilistic::LightHonestPeer,
};
use events::{Event, EventMessage, EventPublisher};
use num_traits::Bounded;
use ordered_float::OrderedFloat;
use primitives::NodeId;
use serde::{Deserialize, Serialize};
use telemetry::info;
use theater::{ActorId, ActorImpl, ActorLabel, ActorState, Handler};

/// A type used to encapsulate Reputation Updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationUpdateEvent {
    pub sender: Option<NodeId>,
    pub peer: NodeId,
    pub delta: f64,
    pub update: Update,
}

/// A configuration struct for the Reputation Module
/// providing the data necessary to construct a new
/// reputation module
///
/// ```
/// use std::{
///     hash::Hash,
///     fmt::Debug,
///     ops::{
///         SubAssign,
///         AddAssign,
///         DivAssign,
///         Add,
///         Sub,
///         Mul,
///         Div
///     }
/// };
///
/// use buckets::bucketize::BucketizeSingle;
/// use events::EventPublisher;
///
/// #[derive(Debug, Clone)]
/// pub struct ReputationModuleConfig<B>
/// where
///     + B: BucketizeSingle<OrderedFloat<f64>> + Clone,
/// {
///     
///    pub reputation_error_bound: f64,
///    pub reputation_probability: f64,
///    pub reputation_max_entries: f64,
///    pub reputation_min: OrderedFloat<f64>,
///    pub reputation_max: OrderedFloat<f64>,
///    pub credit_error_bound: f64,
///    pub credit_probability: f64,
///    pub credit_max_entries: f64,
///    pub credit_min: OrderedFloat<f64>,
///    pub credit_max: OrderedFloat<f64>,
///    pub events_tx: EventPublisher
///    pub bucketizer: B,
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ReputationModuleConfig<B>
where
    B: BucketizeSingle<OrderedFloat<f64>> + Clone,
{
    pub reputation_error_bound: f64,
    pub reputation_probability: f64,
    pub reputation_max_entries: f64,
    pub reputation_min: OrderedFloat<f64>,
    pub reputation_max: OrderedFloat<f64>,
    pub credit_error_bound: f64,
    pub credit_probability: f64,
    pub credit_max_entries: f64,
    pub credit_min: OrderedFloat<f64>,
    pub credit_max: OrderedFloat<f64>,
    pub events_tx: EventPublisher,
    pub bucketizer: B,
}

/// A module for tracking the reputation and message credits of peers
/// in a decentralized, trustless, peer to peer network.
///
/// ```
/// use std::{
///     collections::HashSet,
///     fmt::Debug,
///     hash::Hash,
///     ops::{Add, AddAssign, Div, DivAssign, Mul, Sub, SubAssign},
/// };
///
/// use buckets::bucketize::BucketizeSingle;
/// use decentrust::probabilistic::LightHonestPeer;
/// use events::EventPublisher;
/// use num_traits::Bounded;
/// use theater::{ActorId, ActorImpl, ActorLabel, ActorState, Handler};
///
/// pub struct ReputationModule<B>
/// where
///     B: BucketizeSingle<OrderedFloat<f64>>,
/// {
///     status: ActorState,
///     label: ActorLabel,
///     id: ActorId,
///     events_tx: EventPublisher,
///     reputation: LightHonestPeer<NodeId, OrderedFloat<f64>>,
///     credits: LightHonestPeer<NodeId, OrderedFloat<f64>>,
///     bucketizer: B,
///     peer_set: HashSet<NodeIde>,
/// }
/// ```
pub struct ReputationModule<B>
where
    B: BucketizeSingle<OrderedFloat<f64>>,
{
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: EventPublisher,
    reputation: LightHonestPeer<NodeId, OrderedFloat<f64>>,
    credits: LightHonestPeer<NodeId, OrderedFloat<f64>>,
    bucketizer: B,
    peer_set: HashSet<NodeId>,
}

impl<B> ReputationModule<B>
where
    B: BucketizeSingle<OrderedFloat<f64>> + Clone,
{
    /// Creates a new `ReputationModule` struct with two
    /// `LightHonestPeer` instances and a Bucketizer,
    /// as well as all the Actor required objects.
    /// Also receives an EventPublisher.
    ///
    /// # Example
    ///
    /// ```
    /// use node::runtime::reputation_module::{
    ///     ReputationModuleConfig,
    ///     ReputationModule
    /// };
    /// use buckets::bucketize::BucketizeSingle;
    /// use buckets::bucketizers::FixedWidthBucketizer;
    /// use events::EventPublisher;
    /// use decentrust::probabilistic::LightHonestPeer;
    /// use ordered_float::OrderedFloat;
    /// use num_traits::Bounded;
    /// use theater::{ActorState, ActorId, ActorLabel, ActorImpl, Handler};
    /// use tokio::sync::mpsc::channel;
    ///
    /// let (tx, rx) = channel();
    ///
    /// let bucketizer: FixedWidthBucketizer<OrderedFloat<f64>> = FixedWidthBucketizer::new();
    ///
    /// let config = ReputationModuleConfig<OrderedFloat<f64>> {
    ///     reputation_error_bound: 50.0f64,
    ///     reputation_probability: 0.001f64,
    ///     reputation_max_entries: 10_000.0f64,
    ///     reputation_min: OrderedFloat::from(0.0f64)
    ///     reputation_max: OrderedFloat::from(1000.0f64)
    ///     credit_error_bound: 100.0f64,
    ///     credit_probability: 0.01f64,
    ///     credit_max_entries: 30_000.0f64,
    ///     credit_min: OrderedFloat::from(0.0f64),
    ///     credit_max: OrderedFloat::from(f64::max_value()),
    ///     events_tx: tx.clone(),
    ///     bucketizer,
    /// };
    ///
    /// let reputation_module: ReputationModule<
    ///     &str,
    ///     OrderedFloat<f64>,
    ///     FixedWidthBucketizer<
    ///         OrderedFloat<f64>
    ///     >
    /// > = ReputationModule::new(&config);
    /// ```
    pub fn new(config: ReputationModuleConfig<B>) -> Self {
        let reputation: LightHonestPeer<NodeId, OrderedFloat<f64>> = {
            LightHonestPeer::new_from_bounds(
                config.reputation_error_bound,
                config.reputation_probability,
                config.reputation_max_entries,
                config.reputation_min,
                config.reputation_max,
            )
        };

        let credits: LightHonestPeer<NodeId, OrderedFloat<f64>> = {
            LightHonestPeer::new_from_bounds(
                config.credit_error_bound,
                config.credit_probability,
                config.credit_max_entries,
                config.credit_min,
                config.credit_max,
            )
        };

        ReputationModule {
            id: uuid::Uuid::new_v4().to_string(),
            label: String::from("Reputation"),
            status: ActorState::Stopped,
            events_tx: config.events_tx,
            reputation,
            credits,
            bucketizer: config.bucketizer,
            peer_set: HashSet::new(),
        }
    }

    fn init_local_reputation(&mut self, peer: &NodeId, init_value: OrderedFloat<f64>) {
        self.reputation.init_local(peer, init_value);
    }

    fn update_local_reputation(
        &mut self,
        receiver: &NodeId,
        value: OrderedFloat<f64>,
        update: Update,
    ) {
        self.reputation.update_local(receiver, value, update);
    }

    fn get_reputation_raw_local(&self, key: &NodeId) -> Option<OrderedFloat<f64>> {
        self.reputation.get_raw_local(key)
    }

    fn get_reputation_normalized_local(&self, key: &NodeId) -> Option<OrderedFloat<f64>> {
        self.reputation.get_normalized_local(key)
    }

    fn get_reputation_raw_local_map(&self) -> CountMinSketch<OrderedFloat<f64>> {
        self.reputation.get_raw_local_map()
    }

    fn get_reputation_normalized_local_map(&self) -> CountMinSketch<OrderedFloat<f64>> {
        self.reputation.get_normalized_local_map()
    }

    fn init_global_reputation(
        &mut self,
        sender: &NodeId,
        peer: &NodeId,
        init_value: OrderedFloat<f64>,
    ) {
        self.reputation.init_global(sender, peer, init_value)
    }

    fn update_global_reputation(
        &mut self,
        sender: &NodeId,
        receiver: &NodeId,
        value: OrderedFloat<f64>,
        update: Update,
    ) {
        self.reputation
            .update_global(sender, receiver, value, update);
    }

    fn get_reputation_raw_global(&self, key: &NodeId) -> Option<OrderedFloat<f64>> {
        self.reputation.get_raw_global(key)
    }

    fn get_reputation_normalized_global(&self, key: &NodeId) -> Option<OrderedFloat<f64>> {
        self.reputation.get_normalized_global(key)
    }

    fn get_reputation_global_local_map(&self) -> CountMinSketch<OrderedFloat<f64>> {
        self.reputation.get_raw_global_map()
    }

    fn get_reputation_normalized_global_map(&self) -> CountMinSketch<OrderedFloat<f64>> {
        self.reputation.get_normalized_global_map()
    }

    fn bucketize_reputation_raw_local(&self) -> impl Iterator<Item = (NodeId, usize)> + '_ {
        self.reputation
            .bucketize_local(self.peer_set.clone().into_iter(), self.bucketizer.clone())
    }

    fn bucketize_reputation_normalized_local(&self) -> impl Iterator<Item = (NodeId, usize)> + '_ {
        self.reputation
            .bucketize_normalized_local(self.peer_set.clone().into_iter(), self.bucketizer.clone())
    }

    fn bucketize_reputation_raw_global(&self) -> impl Iterator<Item = (NodeId, usize)> + '_ {
        self.reputation
            .bucketize_global(self.peer_set.clone().into_iter(), self.bucketizer.clone())
    }

    fn bucketize_reputation_normalized_global(&self) -> impl Iterator<Item = (NodeId, usize)> + '_ {
        self.reputation
            .bucketize_normalized_global(self.peer_set.clone().into_iter(), self.bucketizer.clone())
    }

    fn init_local_credit(&mut self, peer: &NodeId, init_value: OrderedFloat<f64>) {
        self.credits.init_local(peer, init_value);
    }

    fn update_local_credit(&mut self, receiver: &NodeId, value: OrderedFloat<f64>, update: Update) {
        self.credits.update_local(receiver, value, update);
    }

    fn get_credits_raw_local(&self, key: &NodeId) -> Option<OrderedFloat<f64>> {
        self.credits.get_raw_local(key)
    }

    fn get_credits_normalized_local(&self, key: &NodeId) -> Option<OrderedFloat<f64>> {
        self.credits.get_normalized_local(key)
    }

    fn get_credits_raw_local_map(&self) -> CountMinSketch<OrderedFloat<f64>> {
        self.credits.get_raw_local_map()
    }

    fn get_credits_normalized_local_map(&self) -> CountMinSketch<OrderedFloat<f64>> {
        self.credits.get_normalized_local_map()
    }

    fn init_global_credits(
        &mut self,
        sender: &NodeId,
        peer: &NodeId,
        init_value: OrderedFloat<f64>,
    ) {
        self.credits.init_global(sender, peer, init_value)
    }

    fn update_global_credits(
        &mut self,
        sender: &NodeId,
        receiver: &NodeId,
        value: OrderedFloat<f64>,
        update: Update,
    ) {
        self.credits.update_global(sender, receiver, value, update);
    }

    fn get_credits_raw_global(&self, key: &NodeId) -> Option<OrderedFloat<f64>> {
        self.credits.get_raw_global(key)
    }

    fn get_credits_normalized_global(&self, key: &NodeId) -> Option<OrderedFloat<f64>> {
        self.credits.get_normalized_global(key)
    }

    fn get_credits_global_local_map(&self) -> CountMinSketch<OrderedFloat<f64>> {
        self.credits.get_raw_global_map()
    }

    fn get_credits_normalized_global_map(&self) -> CountMinSketch<OrderedFloat<f64>> {
        self.credits.get_normalized_global_map()
    }
}


#[async_trait]
impl<B> Handler<EventMessage> for ReputationModule<B>
where
    B: BucketizeSingle<OrderedFloat<f64>> + Clone,
{
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.label.clone()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_start(&self) {
        info!("{}-{} starting", self.label(), self.id(),);
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.label(),
            self.id(),
        );
    }

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            Event::RepUpdate(update_bytes) => {
                if let Ok(rep_update) =
                    serde_json::from_slice::<ReputationUpdateEvent>(&update_bytes)
                {
                    if let Some(sender) = rep_update.sender {
                        self.update_global_reputation(
                            &sender,
                            &rep_update.peer,
                            rep_update.delta.into(),
                            rep_update.update,
                        )
                    } else {
                        self.update_local_reputation(
                            &rep_update.peer,
                            rep_update.delta.into(),
                            rep_update.update,
                        )
                    }
                }
            },
            Event::InitRepUpdate(update_bytes) => {
                if let Ok(rep_update) =
                    serde_json::from_slice::<ReputationUpdateEvent>(&update_bytes)
                {
                    if let Some(sender) = rep_update.sender {
                        self.init_global_reputation(
                            &sender,
                            &rep_update.peer,
                            rep_update.delta.into(),
                        );
                    } else {
                        self.init_local_reputation(&rep_update.peer, rep_update.delta.into());
                    }
                }
            },
            Event::NoOp => {},
            _ => {},
        }
        Ok(ActorState::Running)
    }
}

unsafe impl<B> Send for ReputationModule<B> where B: BucketizeSingle<OrderedFloat<f64>> + Clone {}
