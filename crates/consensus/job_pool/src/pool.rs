//! Implementation of the Job pool that will be used by Job Scheduler.

use std::{
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
        Condvar,
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use concurrent_queue::ConcurrentQueue;
use crossbeam_channel::{Receiver, Sender};

use crate::{
    task::{Job, Task},
    worker::{JobListener, Worker},
};

/// A Job pool for running multiple job on a configurable group of workers
pub struct JobPool {
    pub name: String,
    pub stack_size: Option<usize>,
    pub concurrency_limit: usize,
    pub waiting_queue: (Sender<Job>, Receiver<Job>),
    // The jobs send to this queue are for those idle workers who are polling for work
    pub immediate_job_queue: (Sender<Job>, Receiver<Job>),
    pub state: Arc<State>,
}

impl JobPool {
    /// Get the number of jobs currently in the job pool.
    pub fn jobs(&self) -> usize {
        if let Ok(jobs_count) = self.state.jobs_count.lock() {
            return *jobs_count;
        }
        0
    }

    pub fn queued_tasks(&self) -> usize {
        self.waiting_queue.0.len()
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn running_tasks(&self) -> usize {
        self.state.running_jobs_count.load(Ordering::Relaxed)
    }

    /// Submit a job to be executed by the Job pool.
    pub fn run_sync_job<T, F>(&self, closure: F) -> Task<T>
    where
        T: Send + Sync + 'static,
        F: FnOnce() -> T + Send + Sync + 'static,
    {
        let (task, job) = Task::from_closure(closure);
        self.execute_job(job);
        task
    }

    /// Submit a future to be executed by the job pool.
    pub fn run_async_job<T, F>(&self, future: F) -> Task<T>
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let (task, job) = Task::from_future(future);
        self.execute_job(job);
        task
    }

    fn execute_job(&self, job: Job) {
        if let Err(job) = self.try_execute_job(job) {
            let _ = self.waiting_queue.0.send(job);
        }
    }

    fn try_execute_job(&self, job: Job) -> Result<(), Job> {
        // Send the job to the idle worker polling for the job
        if let Err(e) = self.immediate_job_queue.0.try_send(job) {
            // No Workers are currently polling the queue,Addition worker can be spawn if
            // No of workers < no of max jobs allowed
            // else the jobs are further pushed down to the waiting
            // If possible, spawn an additional thread to handle the task.
            if let Err(Some(job)) = self.spawn_job(Some(e.into_inner())) {
                if let Err(e) = self.waiting_queue.0.try_send(job) {
                    return Err(e.into_inner());
                }
            }
        }

        Ok(())
    }

    /// Gracefully Shut down this Job pool i.e to block until all existing Jobs
    /// have completed
    pub fn join(self) {
        self.join_deadline(None);
    }

    /// Shut down this job pool and block until all existing tasks have
    /// completed and workers have stopped, or the given deadline passes.
    ///
    /// Returns `true` if the job pool shut down fully before the deadline.
    /// Arguments:
    ///
    /// * `timeout`: The amount of time to wait for the before Pool is shutdown.
    ///
    /// Returns:
    ///
    /// A boolean value.
    pub fn join_timeout(self, timeout: Duration) -> bool {
        self.join_deadline(Some(Instant::now() + timeout))
    }

    fn join_deadline(self, deadline: Option<Instant>) -> bool {
        //inform all workers both running as well as idle,that pool is shutting down
        drop(self.waiting_queue.0);
        if let Ok(mut workers_count) = self.state.jobs_count.lock() {
            if deadline.is_none() {
                if let Ok(waiting_workers) = self.state.shutdown.wait(workers_count) {
                    //todo: this reassignment is never read, not sure what the intention is...
                    workers_count = waiting_workers;
                };
            } else {
                // Graceful shutdown of workers from the pool.
                while *workers_count > 0 {
                    if let Some(deadline) = deadline {
                        if let Some(timeout) = deadline.checked_duration_since(Instant::now()) {
                            let value = self.state.shutdown.wait_timeout(workers_count, timeout);
                            match value {
                                Ok(value) => {
                                    workers_count = value.0;
                                    if value.1.timed_out() {
                                        return false;
                                    }
                                },
                                Err(e) => {
                                    workers_count = e.into_inner().0;
                                },
                            }
                        } else {
                            return false;
                        }
                    }
                }
            }
            return true;
        }
        false
    }

    /// Spawn an additional job from job  pool,
    pub fn spawn_job(&self, task: Option<Job>) -> Result<(), Option<Job>> {
        if let Ok(mut jobs_count) = self.state.jobs_count.lock() {
            if *jobs_count >= self.state.max_jobs {
                return Err(task);
            }
            // Configure the job based on the job pool configuration.
            let mut builder = thread::Builder::new().name(self.name.clone());
            if let Some(size) = self.stack_size {
                builder = builder.stack_size(size);
            }
            *jobs_count += 1;
            let worker = Worker::new(
                task,
                self.waiting_queue.1.clone(),
                self.immediate_job_queue.1.clone(),
                self.concurrency_limit,
                self.state.keep_alive,
                WorkerListener {
                    state: self.state.clone(),
                },
            );
            drop(jobs_count);
            let _ = builder.spawn(move || worker.run_job());
            return Ok(());
        }
        Err(task)
    }
}

struct WorkerListener {
    state: Arc<State>,
}

impl JobListener for WorkerListener {
    fn on_job_started(&mut self) {
        self.state
            .running_jobs_count
            .fetch_add(1, Ordering::Relaxed);
    }

    fn on_job_finished(&mut self, job_completion_duration: u128, is_async: bool, is_error: bool) {
        self.state
            .running_jobs_count
            .fetch_sub(1, Ordering::Relaxed);

        // Report the Job Completion for calculating Alpha Gamma for back pressure
        if !is_async && !is_error {
            if self.state.tasks_completion_time.len() > self.state.no_of_task_time_to_track {
                let no_of_task_times_to_truncate = self
                    .state
                    .tasks_completion_time
                    .len()
                    .saturating_sub(self.state.no_of_task_time_to_track);
                for _i in 0..no_of_task_times_to_truncate {
                    let _ = self.state.tasks_completion_time.pop();
                }
            }
            let _ = self
                .state
                .tasks_completion_time
                .push(job_completion_duration);
        }
    }

    fn on_idle(&mut self) -> bool {
        if let Ok(jobs_count) = self.state.jobs_count.lock() {
            return *jobs_count > self.state.min_jobs;
        }
        false
    }
}

impl Drop for WorkerListener {
    fn drop(&mut self) {
        if let Ok(mut count) = self.state.jobs_count.lock() {
            *count = count.saturating_sub(1);
            self.state.shutdown.notify_all();
        }
    }
}

/// Job pool state shared by the Workers
pub struct State {
    pub min_jobs: usize,
    pub max_jobs: usize,
    pub jobs_count: Mutex<usize>,
    pub running_jobs_count: AtomicUsize,
    pub keep_alive: Duration,
    pub shutdown: Condvar,
    pub no_of_task_time_to_track: usize,
    //Keep Track of n completion time for jobs
    pub tasks_completion_time: ConcurrentQueue<u128>,
}
