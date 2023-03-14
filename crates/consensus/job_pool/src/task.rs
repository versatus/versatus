//! Implementation of a task, as well as underlying primitives used to drive
//! their execution.

use std::{
    future::Future,
    panic::resume_unwind,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::{Duration, Instant},
};

use crate::{error::PoolError, poller::JobPoller};

// Type that is returned after job is submitted to the pool
pub struct Task<T> {
    status: Arc<Mutex<TaskStatus<T>>>,
}

pub struct TaskStatus<T> {
    pub result: Option<thread::Result<T>>,
    //A Waker is a handle for waking up a task by notifying its executor that it is ready to be
    // run.
    pub waker: Option<Waker>,
    pub is_timeout: bool,
}

impl<T> Task<T> {
    /// Create a new task for  sync job.
    ///
    /// Arguments:
    ///
    /// * `closure`: The closure to run.
    pub fn from_closure<F>(closure: F) -> (Self, Job)
    where
        F: FnOnce() -> T + Send + Sync + 'static,
        T: Send + 'static,
    {
        let task = Self::create_pending_task();
        let job = Job {
            is_future: false,
            waker: crate::waker::empty_waker(),
            poller: Box::new(SyncJobPoller {
                sync_job: Some(closure),
                result: None,
                task: task.status.clone(),
            }),
        };
        (task, job)
    }

    ///  Create a new task for asynchronous job
    /// It takes a future, creates a task, and returns both
    ///
    /// Arguments:
    ///
    /// * `future`: The future that we want to run.
    pub fn from_future<F>(future: F) -> (Self, Job)
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let task = Self::create_pending_task();
        let job = Job {
            is_future: true,
            waker: crate::waker::empty_waker(),
            poller: Box::new(AsyncJobPoller {
                future,
                result: None,
                task: task.status.clone(),
            }),
        };

        (task, job)
    }

    /// `create_pending_task` creates a new `Task` with a `status` field that is
    /// an `Arc` of a `Mutex` of a `TaskStatus` that has a `result` field
    /// that is a `None` and a `waker` field that is also a `None`
    ///
    /// Returns:
    ///
    /// A `Task` struct.
    fn create_pending_task() -> Self {
        Self {
            status: Arc::new(Mutex::new(TaskStatus {
                result: None,
                waker: None,
                is_timeout: false,
            })),
        }
    }

    /// Check if the task exited because of timeout
    pub fn has_timeout_occurred(&self) -> bool {
        let has_timed_out = if let Ok(status) = self.status.lock() {
            status.is_timeout
        } else {
            false
        };
        has_timed_out
    }

    /// Check if the task is done yet.

    pub fn is_finished(&self) -> bool {
        let has_finished = if let Ok(status) = self.status.lock() {
            status.result.is_some()
        } else {
            false
        };
        has_finished
    }

    /// Block the current thread until the task completes and return the value
    /// the task produced.
    ///
    /// # Panics
    ///
    /// If the underlying task panics, the panic will propagate to this call.
    pub fn join(self) -> Result<T, PoolError> {
        match self.join_catch() {
            Some(Ok(value)) => Ok(value),
            Some(Err(e)) => resume_unwind(e),
            _ => Err(PoolError::FailedToEndTask),
        }
    }

    fn join_catch(self) -> Option<thread::Result<T>> {
        if let Ok(mut status) = self.status.lock() {
            let result = if let Some(result) = status.result.take() {
                result
            } else {
                status.waker = Some(crate::waker::unpark_current_thread());
                drop(status);
                loop {
                    thread::park();
                    if let Ok(mut status) = self.status.lock() {
                        if let Some(result) = status.result.take() {
                            break result;
                        }
                    }
                }
            };
            return Some(result);
        };
        None
    }

    /// Block the current worker until the task completes or a timeout is
    /// reached.
    ///
    /// # Panics
    ///
    /// If the underlying task panics, the panic will propagate to this call.
    pub fn join_timeout(self, timeout: Duration) -> Result<Result<T, Self>, PoolError> {
        match self.join_deadline(Instant::now() + timeout) {
            None => Err(PoolError::FailedToEndTask),
            Some(task) => Ok(task),
        }
    }

    /// Block the current worker until the task completes or a timeout is
    pub fn join_deadline(self, deadline: Instant) -> Option<Result<T, Self>> {
        if let Ok(mut status) = self.status.clone().lock() {
            match {
                if let Some(result) = status.result.take() {
                    Some(result)
                } else {
                    status.waker = Some(crate::waker::unpark_current_thread());
                    drop(status);
                    Some(loop {
                        if let Some(timeout) = deadline.checked_duration_since(Instant::now()) {
                            thread::park_timeout(timeout);
                            if let Ok(mut status) = self.status.as_ref().lock() {
                                status.is_timeout = true;
                            }
                        } else {
                            return Some(Err(self));
                        }
                        if let Ok(mut status) = self.status.as_ref().lock() {
                            if let Some(result) = status.result.take() {
                                break result;
                            }
                        }
                    })
                }
            } {
                Some(Ok(value)) => return Some(Ok(value)),
                Some(Err(e)) => resume_unwind(e),
                _ => {
                    return None;
                },
            }
        }
        None
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Ok(mut inner) = self.status.lock() {
            return match inner.result.take() {
                Some(Ok(value)) => Poll::Ready(value),
                Some(Err(e)) => resume_unwind(e),
                None => {
                    inner.waker = Some(cx.waker().clone());
                    Poll::Pending
                },
            };
        }
        Poll::Pending
    }
}

/// A `Job` is a future that will be polled by a `JobPoller`.
///
/// The `is_future` field is a boolean that indicates whether the `Job` is a
/// future or not.
///
/// The `waker` field is a `Waker` that will be used to wake up the `Job` when
/// it's ready to be polled.
///
/// The `poller` field is a `Box<dyn JobPoller>` that will be used to poll the
/// `Job`.
///
/// Properties:
///
/// * `is_future`: This is a boolean that indicates whether the job is a future
///   or not.
/// * `waker`: A waker is a handle to a thread that can be used to wake it up.
/// * `poller`: This is the job that will be polled.
pub struct Job {
    is_future: bool,
    waker: Waker,
    poller: Box<dyn JobPoller>,
}

impl Job {
    /// Determine whether this task is async
    pub fn is_async(&self) -> bool {
        self.is_future
    }

    /// Get the unique memory address for this job.
    pub fn addr(&self) -> usize {
        &*self.poller as *const dyn JobPoller as *const () as usize
    }

    /// Set the waker to use with this task.
    pub fn set_waker(&mut self, waker: Waker) {
        self.waker = waker;
    }

    /// Run the job until it yields or completes.
    /// Once this function returns `Complete` it should not be called again.
    pub fn run(&mut self) -> JobExecutionStatus {
        let mut cx = Context::from_waker(&self.waker);
        self.poller.run_job(&mut cx)
    }

    /// Complete the task and update its state related to the job.
    pub fn complete(mut self) {
        self.poller.complete();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobExecutionStatus {
    //For Async Jobs
    Yield,
    Complete { is_err: bool },
    NoJobToPoll,
}

/// `SyncJobPoller` is a type that holds a `sync_job` and a `task` and polls the
/// `task` to see if the `sync_job` has completed.
///
/// Properties:
///
/// * `sync_job`: This is the job that will be executed in the background.
/// * `result`: This is the result of the thread.
/// * `task`: This is the task that we're going to poll.

pub struct SyncJobPoller<F, T> {
    pub sync_job: Option<F>,
    pub result: Option<thread::Result<T>>,
    pub task: Arc<Mutex<TaskStatus<T>>>,
}

/// It's a Poller that polls the future, a result, and a task.
///
/// Properties:
///
/// * `future`: The future that we're polling.
/// * `result`: This is the result of the future. It's an Option because the
///   future may not have
/// completed yet.
/// * `task`: This is the task that we're going to poll.

pub struct AsyncJobPoller<F, T> {
    pub future: F,
    pub result: Option<thread::Result<T>>,
    pub task: Arc<Mutex<TaskStatus<T>>>,
}
