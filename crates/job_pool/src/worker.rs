use std::{collections::HashMap, time::Duration};

use crossbeam_channel::{unbounded, Receiver, Select, Sender};

use crate::task::{Job, JobExecutionStatus};

/// A type which receives notifications from a worker.
pub trait JobListener {
    fn on_job_started(&mut self) {}

    fn on_job_finished(&mut self, _job_completion_duration: u128, _is_async: bool, _is_err: bool) {}

    fn on_idle(&mut self) -> bool {
        true
    }
}

/// A worker thread which belongs to a job pool and executes tasks.
pub struct Worker<L: JobListener> {
    keep_alive: Duration,

    concurrency_limit: usize,
    /// An initial task this worker should be run before polling for new work.
    initial_job: Option<Job>,

    /// Pending Jobs + Async Job that has yeilded
    pending_jobs: HashMap<usize, Job>,

    /// Queue of new tasks to run. The worker pulls more tasks from this queue
    /// when idle.
    waiting_queue: Receiver<Job>,

    immediate_job_queue: Receiver<Job>,

    /// Channel used to receive notifications from wakers for pending tasks.
    pending_job_notifications: (Sender<usize>, Receiver<usize>),

    /// Set to true when the worker is running and wants to consume more work.
    active: bool,

    /// Receiver of various worker events.
    listener: L,
}

impl<L: JobListener> Worker<L> {
    /// Create a new worker.
    pub fn new(
        initial_task: Option<Job>,
        queue: Receiver<Job>,
        immediate_queue: Receiver<Job>,
        concurrency_limit: usize,
        keep_alive: Duration,
        listener: L,
    ) -> Self {
        Self {
            keep_alive,
            concurrency_limit,
            initial_job: initial_task,
            pending_jobs: HashMap::new(),
            waiting_queue: queue,
            immediate_job_queue: immediate_queue,
            pending_job_notifications: unbounded(),
            active: false,
            listener,
        }
    }

    /// Run the worker on the current thread until the work queue is closed.
    pub fn run_job(mut self) {
        self.active = true;

        if let Some(job) = self.initial_job.take() {
            self.execute_job(job);
        }

        // Main worker , keep running until the pool shuts down and pending jobs have
        // finished.
        while self.active || !self.pending_jobs.is_empty() {
            match self.poll_work() {
                PollingStatus::New(job) => self.execute_job(job),
                PollingStatus::Unpark(id) => self.finish_pending_job(id),
                PollingStatus::ShutDown => self.active = false,
                PollingStatus::Timeout => {
                    if self.pending_jobs.is_empty() && self.listener.on_idle() {
                        self.active = false;
                    }
                },
                PollingStatus::Busy => {
                    std::thread::sleep(Duration::from_millis(500));
                    //Then Poll again
                },
            }
        }
    }

    /// Poll for the next work item the worker should work on.
    fn poll_work(&mut self) -> PollingStatus {
        let mut queue_id = None;
        let mut immediate_queue_id = None;
        let mut pending_job_id = None;
        let mut select = Select::new();

        if self.active && self.pending_jobs.len() < self.concurrency_limit {
            queue_id = Some(select.recv(&self.waiting_queue));
            immediate_queue_id = Some(select.recv(&self.immediate_job_queue));
        }

        // Check for pending jobs
        if !self.pending_jobs.is_empty() {
            pending_job_id = Some(select.recv(&self.pending_job_notifications.1));
        }

        match select.select_timeout(self.keep_alive) {
            Ok(op) if Some(op.index()) == queue_id => {
                if let Ok(job) = op.recv(&self.waiting_queue) {
                    PollingStatus::New(job)
                } else {
                    PollingStatus::ShutDown
                }
            },
            Ok(op) if Some(op.index()) == immediate_queue_id => {
                if let Ok(job) = op.recv(&self.immediate_job_queue) {
                    PollingStatus::New(job)
                } else {
                    PollingStatus::ShutDown
                }
            },
            Ok(op) if Some(op.index()) == pending_job_id => {
                if let Ok(id) = op.recv(&self.pending_job_notifications.1) {
                    PollingStatus::Unpark(id)
                } else {
                    PollingStatus::Busy
                }
            },
            //If we dont get any notifications,the pool is busy
            Ok(_) => PollingStatus::Busy,
            Err(_) => PollingStatus::Timeout,
        }
    }

    /// > The function takes a job, checks if it's async, and if it is, it sets
    /// > a waker on the job, and
    /// then runs the job. If the job is not async, it just runs the job
    ///
    /// Arguments:
    ///
    /// * `job`: The job to be executed.
    fn execute_job(&mut self, mut job: Job) {
        if job.is_async() {
            let sender = self.pending_job_notifications.0.clone();
            let job_addr = job.addr();
            job.set_waker(waker_fn::waker_fn(move || {
                let _ = sender.send(job_addr);
            }));
        }
        let start_duration = std::time::Instant::now();
        self.listener.on_job_started();

        if let JobExecutionStatus::Complete { is_err } = job.run() {
            let completion_time = start_duration.elapsed().as_millis();
            self.listener
                .on_job_finished(completion_time, job.is_async(), is_err);
            job.complete();
        } else {
            //Rescheduling of pending job
            self.pending_jobs.insert(job.addr(), job);
        }
    }

    ///Run the job until it completes, remove it from the pending jobs list
    ///
    /// Arguments:
    ///
    /// * `id`: The id of the job to finish
    fn finish_pending_job(&mut self, id: usize) {
        if let Some(job) = self.pending_jobs.get_mut(&id) {
            if let JobExecutionStatus::Complete { is_err: _is_err } = job.run() {
                // Job is complete
                if let Some(job) = self.pending_jobs.remove(&id) {
                    job.complete();
                }
            }
        }
    }
}

enum PollingStatus {
    /// New Job has arrived for this worker.
    New(Job),

    /// An existing pending Job has woken.
    Unpark(usize),

    /// No activity occurred within the time limit.
    Timeout,

    /// The Job pool has been shut down.
    ShutDown,

    /// The Job pool is currently busy
    Busy,
}
