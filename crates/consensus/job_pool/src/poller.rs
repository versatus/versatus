use std::{
    future::Future,
    panic::{catch_unwind, AssertUnwindSafe},
    pin::Pin,
    task::{Context, Poll},
};

use crate::task::{AsyncJobPoller, JobExecutionStatus, SyncJobPoller};

pub trait JobPoller: Send + 'static {
    fn run_job(&mut self, cx: &mut Context) -> JobExecutionStatus;
    fn complete(&mut self);
}

impl<F, T> JobPoller for SyncJobPoller<F, T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    fn run_job(&mut self, _cx: &mut Context) -> JobExecutionStatus {
        if let Some(closure) = self.sync_job.take() {
            let result = catch_unwind(AssertUnwindSafe(closure));
            let is_err = result.is_err();
            self.result = Some(result);
            JobExecutionStatus::Complete { is_err }
        } else {
            JobExecutionStatus::NoJobToPoll
        }
    }

    fn complete(&mut self) {
        if let Some(result) = self.result.take() {
            if let Ok(mut task) = self.task.lock() {
                task.result = Some(result);
                if let Some(waker) = task.waker.as_ref() {
                    waker.wake_by_ref();
                };
            }
        }
    }
}

impl<F, T> JobPoller for AsyncJobPoller<F, T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    fn run_job(&mut self, cx: &mut Context) -> JobExecutionStatus {
        let future = unsafe { Pin::new_unchecked(&mut self.future) };
        match catch_unwind(AssertUnwindSafe(|| future.poll(cx))) {
            Ok(Poll::Pending) => JobExecutionStatus::Yield,
            Ok(Poll::Ready(value)) => {
                self.result = Some(Ok(value));
                JobExecutionStatus::Complete { is_err: false }
            },
            Err(e) => {
                self.result = Some(Err(e));
                JobExecutionStatus::Complete { is_err: true }
            },
        }
    }

    /// Updates the task associated with the job.
    fn complete(&mut self) {
        if let Some(result) = self.result.take() {
            if let Ok(mut task) = self.task.lock() {
                task.result = Some(result);
                if let Some(waker) = task.waker.as_ref() {
                    waker.wake_by_ref();
                };
            }
        }
    }
}
