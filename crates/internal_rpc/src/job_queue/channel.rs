use crate::job_queue::{
    job::{ServiceJobApi, ServiceJobState, ServiceJobStatusResponse, ServiceJobType},
    ServiceJobQueue,
};
use std::sync::{Arc, Condvar, Mutex};

pub trait ServiceTransmitter<J: ServiceJobApi>: Send + Sync {
    fn send(&self, cid: &str, kind: ServiceJobType) -> uuid::Uuid;
    fn new(
        all_jobs: &Arc<Mutex<ServiceJobQueue<J>>>,
        store: &Arc<Mutex<ServiceJobQueue<J>>>,
        emitter: &Arc<Condvar>,
    ) -> Self;
    /// Get the status of a job in queue. Takes O(n) time.
    /// Best case scenario, the job in question is the last job in queue,
    /// ie. it's the first item in the vec.
    fn job_status(&self, uuid: uuid::Uuid) -> Option<ServiceJobStatusResponse>;
}

pub trait ServiceReceiver<J: ServiceJobApi>: Send + Sync {
    fn recv(&self) -> Option<J>;
    fn new(
        all_jobs: &Arc<Mutex<ServiceJobQueue<J>>>,
        store: &Arc<Mutex<ServiceJobQueue<J>>>,
        emitter: &Arc<Condvar>,
    ) -> Self;
}

#[derive(Debug)]
pub struct Transmitter<J: ServiceJobApi> {
    /// tracks the status of any job requested by a client
    all_jobs: Arc<Mutex<ServiceJobQueue<J>>>,
    store: Arc<Mutex<ServiceJobQueue<J>>>,
    emitter: Arc<Condvar>,
}
impl<J: ServiceJobApi + Clone> ServiceTransmitter<J> for Transmitter<J> {
    // TODO(@eureka-cpu): Use if let and return an Option<Uuid>
    /// Add a new job to the front of the queue.
    fn send(&self, cid: &str, kind: ServiceJobType) -> uuid::Uuid {
        let uuid = uuid::Uuid::new_v4();
        let job = J::new(cid, uuid, kind);
        self.all_jobs.lock().unwrap().queue.push_front(job.clone());
        self.store
            .lock()
            .expect("failed to get lock for queue")
            .queue
            .push_front(job);
        self.emitter.notify_one();

        uuid
    }
    fn new(
        all_jobs: &Arc<Mutex<ServiceJobQueue<J>>>,
        store: &Arc<Mutex<ServiceJobQueue<J>>>,
        emitter: &Arc<Condvar>,
    ) -> Self {
        Self {
            all_jobs: Arc::clone(all_jobs),
            store: Arc::clone(store),
            emitter: Arc::clone(emitter),
        }
    }
    /// Get the status of a job in queue. Takes O(n) time.
    /// Best case scenario, the job in question is the last job in queue,
    /// ie. it's the first item in the vec.
    fn job_status(&self, uuid: uuid::Uuid) -> Option<ServiceJobStatusResponse> {
        for job in self.all_jobs.lock().unwrap().queue.iter() {
            if job.uuid() == uuid {
                return Some(job.status());
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct Receiver<J: ServiceJobApi> {
    // TODO(@eureka-cpu): Make this more robust
    // possibly by using a HashMap<Uuid, J>
    all_jobs: Arc<Mutex<ServiceJobQueue<J>>>,
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

        let mut job_opt = store.queue.pop_back();
        if let Some(current_job) = &mut job_opt {
            for job in self.all_jobs.lock().unwrap().queue.iter_mut() {
                if job.uuid() == current_job.uuid() {
                    current_job.update_status(ServiceJobState::Received);
                    job.update_status(ServiceJobState::Received);
                    break;
                }
            }
        }
        job_opt
    }
    fn new(
        all_jobs: &Arc<Mutex<ServiceJobQueue<J>>>,
        store: &Arc<Mutex<ServiceJobQueue<J>>>,
        emitter: &Arc<Condvar>,
    ) -> Self {
        Self {
            all_jobs: Arc::clone(all_jobs),
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
        let all_jobs = Arc::new(Mutex::new(ServiceJobQueue::new()));
        let store = Arc::new(Mutex::new(ServiceJobQueue::new()));
        let emitter = Arc::new(Condvar::new());

        Self {
            tx: T::new(&all_jobs, &store, &emitter),
            rx: R::new(&all_jobs, &store, &emitter),
            marker: std::marker::PhantomData,
        }
    }
}
