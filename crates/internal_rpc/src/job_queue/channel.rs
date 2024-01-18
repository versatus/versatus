//! A generic channel module with a built in job queue that uses interfaces to get around
//! the need for `serde` derive macros required by `jsonrpsee`'s proc macro.
use crate::job_queue::{
    job::{ServiceJobApi, ServiceJobState, ServiceJobStatusResponse, ServiceJobType},
    ServiceJobQueue,
};
use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex},
};

use super::job::ServiceJobStatus;

/// The interface for transmitting messages over a [`ServiceQueueChannel`]
pub trait ServiceTransmitter<J: ServiceJobApi>: Send + Sync {
    /// Send a job over a [`ServiceQueueChannel`] and return its UUID
    fn send(&self, cid: &str, kind: ServiceJobType) -> uuid::Uuid;
    fn new(
        state: &Arc<Mutex<HashMap<uuid::Uuid, ServiceJobStatus>>>,
        store: &Arc<Mutex<ServiceJobQueue<J>>>,
        emitter: &Arc<Condvar>,
    ) -> Self;
    /// Get the status of a job
    fn job_status(&self, uuid: uuid::Uuid) -> Option<ServiceJobStatusResponse>;
}

/// The interface for receiving messages over a [`ServiceQueueChannel`]
pub trait ServiceReceiver<J: ServiceJobApi>: Send + Sync {
    /// Wait for a job to be sent over a [`ServiceQueueChannel`] and return
    /// the job when it is received
    fn recv(&self) -> Option<J>;
    fn update_state(&self, job_opt: &Option<J>, state: ServiceJobState);
    fn new(
        state: &Arc<Mutex<HashMap<uuid::Uuid, ServiceJobStatus>>>,
        store: &Arc<Mutex<ServiceJobQueue<J>>>,
        emitter: &Arc<Condvar>,
    ) -> Self;
}

/// The transmitting end of a [`ServiceQueueChannel`]
#[derive(Debug)]
pub struct Transmitter<J: ServiceJobApi> {
    /// tracks the status of any job requested by a client
    state: Arc<Mutex<HashMap<uuid::Uuid, ServiceJobStatus>>>,
    store: Arc<Mutex<ServiceJobQueue<J>>>,
    emitter: Arc<Condvar>,
}
impl<J: ServiceJobApi + Clone> ServiceTransmitter<J> for Transmitter<J> {
    // TODO(@eureka-cpu): Use if let and return an Option<Uuid>
    fn send(&self, cid: &str, kind: ServiceJobType) -> uuid::Uuid {
        let uuid = uuid::Uuid::new_v4();
        let job = J::new(cid, uuid, kind);
        self.state
            .lock()
            .unwrap()
            .insert(job.uuid(), ServiceJobStatus::default());
        self.store
            .lock()
            .expect("failed to get lock for queue")
            .queue
            .push_front(job);
        self.emitter.notify_one();

        uuid
    }
    fn new(
        state: &Arc<Mutex<HashMap<uuid::Uuid, ServiceJobStatus>>>,
        store: &Arc<Mutex<ServiceJobQueue<J>>>,
        emitter: &Arc<Condvar>,
    ) -> Self {
        Self {
            state: Arc::clone(state),
            store: Arc::clone(store),
            emitter: Arc::clone(emitter),
        }
    }
    fn job_status(&self, uuid: uuid::Uuid) -> Option<ServiceJobStatusResponse> {
        self.state
            .lock()
            .unwrap()
            .get(&uuid)
            .and_then(|state| Some(state.report()))
    }
}

/// The receiving end of a [`ServiceQueueChannel`]
#[derive(Debug)]
pub struct Receiver<J: ServiceJobApi> {
    state: Arc<Mutex<HashMap<uuid::Uuid, ServiceJobStatus>>>,
    store: Arc<Mutex<ServiceJobQueue<J>>>,
    emitter: Arc<Condvar>,
}
impl<J: ServiceJobApi> ServiceReceiver<J> for Receiver<J> {
    /// Wait for a job to be queued by a client.
    fn recv(&self) -> Option<J> {
        let mut store = self.store.lock().unwrap();

        while store.queue.is_empty() {
            store = self.emitter.wait(store).unwrap();
        }

        let job_opt = store.queue.pop_back();
        self.update_state(&job_opt, ServiceJobState::Received);

        job_opt
    }
    fn update_state(&self, job_opt: &Option<J>, state: ServiceJobState) {
        if let Some(current_job) = job_opt {
            if let Some(job_state) = self.state.lock().unwrap().get_mut(&current_job.uuid()) {
                job_state.update(state);
            }
        }
    }
    fn new(
        state: &Arc<Mutex<HashMap<uuid::Uuid, ServiceJobStatus>>>,
        store: &Arc<Mutex<ServiceJobQueue<J>>>,
        emitter: &Arc<Condvar>,
    ) -> Self {
        Self {
            state: Arc::clone(state),
            store: Arc::clone(store),
            emitter: Arc::clone(emitter),
        }
    }
}

/// The equivalent of a simple MSPC channel that
/// contains a queue for tracking [`ServiceJob`]s.
#[derive(Debug)]
pub(crate) struct ServiceQueueChannel<
    T: ServiceTransmitter<J>,
    R: ServiceReceiver<J>,
    J: ServiceJobApi,
> {
    pub(crate) tx: T,
    pub(crate) rx: R,
    marker: std::marker::PhantomData<J>,
}
impl<T: ServiceTransmitter<J>, R: ServiceReceiver<J>, J: ServiceJobApi + std::fmt::Debug>
    ServiceQueueChannel<T, R, J>
{
    pub(crate) fn new() -> Self {
        let state = Arc::new(Mutex::new(HashMap::new()));
        let store = Arc::new(Mutex::new(ServiceJobQueue::new()));
        let emitter = Arc::new(Condvar::new());

        Self {
            tx: T::new(&state, &store, &emitter),
            rx: R::new(&state, &store, &emitter),
            marker: std::marker::PhantomData,
        }
    }
}
