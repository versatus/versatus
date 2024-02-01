use self::job::ServiceJobApi;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt};

pub mod channel;
pub mod job;

/// A queue for managing incoming [`ServiceJob`]s.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceJobQueue<J: ServiceJobApi> {
    pub(crate) queue: VecDeque<J>,
}
impl<J: ServiceJobApi + fmt::Debug> ServiceJobQueue<J> {
    pub(crate) fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}
