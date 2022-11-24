use std::{collections::HashMap, slice::SliceIndex};

use once_cell::sync::Lazy;
use threadfin::{builder, ThreadPool};
use primitives::types::PeerID;

pub struct JobScheduler {
    local_pool: Thread
    
    Pool,
    remote_pool: ThreadPool,
    forwarding_pool: ThreadPool,
    local_backpressure: usize,
    peers_back_pressure: HashMap<PeerID, usize>,
}


static NUM_CPU_CORES: Lazy<usize> = Lazy::new(|| num_cpus::get().max(32));

//Cores Allocation Ratio 40:40:20 (Local:Remote:Forwarding)



enum Job {}

impl JobScheduler {
    pub fn new() -> Self {
        let cores_allocation: (usize, usize, usize) = (
            (*NUM_CPU_CORES as f32 * 0.4).ceil() as usize,
            (*NUM_CPU_CORES as f32 * 0.4).ceil() as usize,
            (*NUM_CPU_CORES as f32 * 0.2).ceil() as usize,
        );

        JobScheduler {
            local_pool: builder().size(cores_allocation.0).build(),
            remote_pool: builder().size(cores_allocation.1).build(),
            forwarding_pool: builder().size(cores_allocation.2).build(),
            local_backpressure: 0,
            peers_back_pressure: HashMap::default(),
        }
    }

    pub fn record_local_backpressure(&mut self) {
        let val=self.local_pool.queued_tasks()+self.local_pool.running_tasks();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
