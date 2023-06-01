use std::{cmp::Ordering, sync::Arc};

use job_pool::{
    builder::PoolBuilder,
    pool::{JobPool, State},
};
use once_cell::sync::Lazy;
use primitives::PeerId as PeerID;
use vrrb_core::cache::Cache;

pub struct JobScheduler {
    local_peer_id: PeerID,
    local_pool: JobPool,
    remote_pool: JobPool,
    forwarding_pool: JobPool,
    peers_back_pressure: Cache<PeerID, f32>,
}

#[derive(Debug, Clone)]
pub struct BackPressure {
    pub peer_id: Vec<u8>,
    pub back_pressure: f32,
}

impl BackPressure {
    fn new(peer_id: Vec<u8>, back_pressure: f32) -> Self {
        BackPressure {
            peer_id,
            back_pressure,
        }
    }

    fn log_normalized_backpressure(mut data: Vec<BackPressure>) -> Vec<BackPressure> {
        let mut new_backpressure_list = Vec::new();
        data.sort_by(|a, b| a.back_pressure.total_cmp(&b.back_pressure));
        if data.len() > 2 {
            let mut minimum_backpressure = 0.0;
            let mut max_backpressure = 0.0;
            if let Some(min) = data.get(0) {
                minimum_backpressure = min.back_pressure;
            }
            if let Some(max) = data.last() {
                max_backpressure = max.back_pressure;
            }
            if max_backpressure > 0.0 && minimum_backpressure > 0.0 {
                for peer_back_pressure in data.iter() {
                    new_backpressure_list.push(BackPressure::new(
                        peer_back_pressure.peer_id.clone(),
                        (peer_back_pressure.back_pressure - minimum_backpressure)
                            / (max_backpressure - minimum_backpressure),
                    ))
                }
            }
        }
        new_backpressure_list
    }
}

static NUM_CPU_CORES: Lazy<usize> = Lazy::new(|| num_cpus::get().max(32));

impl JobScheduler {
    pub fn add_peer_back_pressure(&mut self, peer_id: PeerID, back_pressure: f32) {
        self.peers_back_pressure.push(peer_id, back_pressure);
    }

    /// > The function creates a new JobScheduler object with three thread
    /// > pools, one for local jobs,
    /// one for remote jobs, and one for forwarding jobs
    ///
    /// Arguments:
    ///
    /// * `peer_id`: PeerID
    ///
    /// Returns:
    ///
    /// A JobScheduler struct
    pub fn new(peer_id: PeerID) -> Self {
        let cores_allocation: (usize, usize, usize) = (
            (*NUM_CPU_CORES as f32 * 0.4).ceil() as usize,
            (*NUM_CPU_CORES as f32 * 0.4).ceil() as usize,
            (*NUM_CPU_CORES as f32 * 0.2).ceil() as usize,
        );

        JobScheduler {
            local_peer_id: peer_id,
            local_pool: PoolBuilder::with_workers_capacity(
                cores_allocation.0,
                cores_allocation.0 + 4,
            )
            .unwrap_or(PoolBuilder::default())
            .stack_size(2 * 1024 * 1024)
            .build(),
            remote_pool: PoolBuilder::with_workers_capacity(
                cores_allocation.1,
                cores_allocation.1 + 4,
            )
            .unwrap_or(PoolBuilder::default())
            .stack_size(2 * 1024 * 1024)
            .build(),
            forwarding_pool: PoolBuilder::with_workers_capacity(
                cores_allocation.2,
                cores_allocation.2 + 2,
            )
            .unwrap_or(PoolBuilder::default())
            .stack_size(2 * 1024 * 1024)
            .build(),
            peers_back_pressure: Cache::new(1000, 50000),
        }
    }

    /// > This function returns the average completion time for jobs in the
    /// > local, remote, and
    /// forwarding pools
    pub fn get_avg_completion_times(&self) -> (f32, f32, f32) {
        (
            self.get_avg_job_completion_time(self.local_pool.state.clone()),
            self.get_avg_job_completion_time(self.remote_pool.state.clone()),
            self.get_avg_job_completion_time(self.forwarding_pool.state.clone()),
        )
    }

    fn get_avg_job_completion_time(&self, state: Arc<State>) -> f32 {
        let mut avg_completion_time = 1.0;
        let num_tasks = state.tasks_completion_time.len();
        let mut completion_times = vec![];
        for _ in 0..num_tasks {
            if let Ok(completion_time) = state.tasks_completion_time.pop() {
                completion_times.push(completion_time);
            }
        }
        if !completion_times.is_empty() {
            avg_completion_time =
                completion_times.iter().sum::<u128>() as f32 / completion_times.len() as f32;
        }
        avg_completion_time
    }

    pub fn check_data_locality(&self) -> bool {
        true
    }

    /// > This function calculates the back pressure of the local node and all
    /// > the peers it is connected
    /// to
    ///
    /// Returns:
    ///
    /// A vector of sorted log normalized back pressure values.
    pub fn calculate_back_pressure(&self) -> (BackPressure, Vec<BackPressure>) {
        let (local_time, remote_time, forwarding_pool_time) = self.get_avg_completion_times();
        let local_backpressure = (self.local_pool.queued_tasks() + self.local_pool.running_tasks())
            as f32
            * local_time
            + (self.remote_pool.queued_tasks() + self.remote_pool.running_tasks()) as f32
                * remote_time
            + (self.forwarding_pool.queued_tasks() + self.forwarding_pool.running_tasks()) as f32
                * forwarding_pool_time;
        let mut back_pressure_list = vec![BackPressure::new(
            self.local_peer_id.clone(),
            1.0 + local_backpressure.log10(),
        )];
        for data in self.peers_back_pressure.cache.to_owned().iter() {
            back_pressure_list.push(BackPressure::new(data.0.clone(), 1.0 + data.1.log10()));
        }
        let mut log_normalized_backpressure =
            BackPressure::log_normalized_backpressure(back_pressure_list);
        log_normalized_backpressure.sort_by(|a, b| {
            if let Some(ordering) = a.back_pressure.partial_cmp(&b.back_pressure) {
                ordering
            } else {
                Ordering::Equal
            }
        });

        (
            BackPressure::new(vec![1u8], 1.0 + local_backpressure.log10()),
            log_normalized_backpressure,
        )
    }

    pub fn get_local_pool(&self) -> &JobPool {
        &self.local_pool
    }

    pub fn set_local_pool(&mut self, pool: JobPool) {
        self.local_pool = pool
    }

    pub fn get_remote_pool(&self) -> &JobPool {
        &self.remote_pool
    }

    pub fn set_remote_pool(&mut self, pool: JobPool) {
        self.remote_pool = pool
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicUsize, Ordering::Relaxed},
        thread,
        time::Duration,
    };

    use rand::Rng;

    use super::*;

    #[test]
    /// The function creates a job scheduler with two peers, and then pushes 300
    /// jobs to the local pool. After 200 jobs are pushed to the local pool,
    /// the main thread calculates backpressure. The function asserts that
    /// the backpressure list is not empty, and that the backpressure values are
    /// between 0.0 and 1.0
    fn test_calculate_back_pressure() {
        let mut job_scheduler = JobScheduler::new(vec![2u8]);
        let mut range = rand::thread_rng();
        for i in 0..30u8 {
            job_scheduler
                .peers_back_pressure
                .push(vec![i], range.gen_range(110.0..350.9));
        }
        let num_done = AtomicUsize::new(0);
        thread::scope(|s| {
            s.spawn(|| {
                for i in 0..300 {
                    job_scheduler
                        .local_pool
                        .run_async_job(async { thread::sleep(Duration::from_millis(120)) });
                    num_done.store(i + 1, Relaxed);
                }
            });
            // The main thread calculates backpressure after 200 jobs are pushed to pool.
            loop {
                let n = num_done.load(Relaxed);
                if n >= 300 {
                    let back_pressure_list = job_scheduler.calculate_back_pressure();
                    assert!(back_pressure_list.1.len() > 0);
                    let minimum_backpressure = back_pressure_list.1.get(0).unwrap().back_pressure;
                    let max_backpressure = back_pressure_list
                        .1
                        .get(back_pressure_list.1.len() - 1)
                        .unwrap()
                        .back_pressure;
                    assert!(minimum_backpressure >= 0.0 && max_backpressure <= 1.0);
                    break;
                }
                if n == 300 {
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }
        });
    }
}
