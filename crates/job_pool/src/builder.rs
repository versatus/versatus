use std::{
    sync::{Arc, Condvar},
    time::Duration,
};

use crossbeam_channel::{bounded, unbounded};
use uuid::Uuid;

use crate::pool::{JobPool, State};

/// A builder for constructing a customized [`JobPool`].
///
/// # Examples
///
/// ```
/// let custom_pool = job_pool::builder::PoolBuilder::with_workers_capacity(2, 4).build();
/// ```
#[derive(Debug)]
pub struct PoolBuilder {
    size: Option<(usize, usize)>,
    stack_size: Option<usize>,
    job_queue_size: Option<usize>,
    concurrent_jobs_limit: usize,
    keep_alive: Duration,
    no_completed_tasks_to_track: usize,
}

impl PoolBuilder {
    pub fn with_workers_capacity(min_workers: usize, max_workers: usize) -> Self {
        if min_workers > max_workers {
            panic!("Job pool minimum size cannot be larger than maximum size");
        }

        if max_workers == 0 {
            panic!("Job pool maximum size must be greater than zero");
        }
        Self {
            size: Some((min_workers, max_workers)),
            stack_size: None,
            job_queue_size: None,
            concurrent_jobs_limit: 16,
            keep_alive: Duration::from_secs(60),
            no_completed_tasks_to_track: 100,
        }
    }

    /// set call stack size for Jobs in job pool.Max Size for Rust Thread is 2
    /// MB. # Examples
    ///
    /// ```
    /// // Workers will have a stack size of at least 32 KiB,and Max 2MB.
    /// use job_pool::builder::PoolBuilder;
    /// let pool = PoolBuilder::with_workers_capacity(1,2,).stack_size(2 * 1024*1024).build();

    /// ```
    pub fn stack_size(mut self, size: usize) -> Self {
        self.stack_size = Some(size);
        self
    }

    /// `job_queue_size` is a function to set max pending jobs for pool
    ///
    /// Arguments:
    ///
    /// * `limit`: The maximum number of jobs that can be queued up at any given
    ///   time.
    /// If set to zero, queueing will be disabled and attempting to execute a
    /// new task will block until an idle worker thread can immediately begin
    /// executing the task or a new worker thread can be created to execute the
    /// task.
    ///
    /// Returns:
    ///
    /// A reference to the struct
    pub fn job_queue_size(mut self, limit: usize) -> Self {
        self.job_queue_size = Some(limit);
        self
    }

    /// Set a duration for how long to idle time for an idle worker.
    pub fn keep_alive(mut self, duration: Duration) -> Self {
        self.keep_alive = duration;
        self
    }

    /// `concurrent_jobs_limit` sets the maximum number of jobs a work can run
    /// at any given time
    pub fn concurrent_jobs_limit(mut self, limit: usize) -> Self {
        self.concurrent_jobs_limit = limit;
        self
    }

    /// `no_completed_tasks_to_track` sets the maximum no of completion tasks to
    /// track to calculate Bayesian average completion time for pool.
    pub fn no_completed_tasks_to_track(mut self, num: usize) -> Self {
        self.no_completed_tasks_to_track = num;
        self
    }

    /// Create a Job pool according to the configuration set
    pub fn build(self) -> JobPool {
        let size = self.size.unwrap_or((1, 4));
        let shared = State {
            min_jobs: size.0,
            max_jobs: size.1,
            jobs_count: Default::default(),
            running_jobs_count: Default::default(),
            keep_alive: self.keep_alive,
            shutdown: Condvar::new(),
            no_of_task_time_to_track: self.no_completed_tasks_to_track,
            tasks_completion_time: concurrent_queue::ConcurrentQueue::unbounded(),
        };
        let pool = JobPool {
            name: Uuid::new_v4().to_string(),
            stack_size: self.stack_size,
            concurrency_limit: self.concurrent_jobs_limit,
            waiting_queue: self.job_queue_size.map(bounded).unwrap_or_else(unbounded),
            immediate_job_queue: bounded(0),
            state: Arc::new(shared),
        };

        for _ in 0..size.0 {
            let result = pool.spawn_job(None);
            assert!(result.is_ok());
        }

        pool
    }
}
