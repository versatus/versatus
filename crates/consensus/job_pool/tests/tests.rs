use std::{thread, time::Duration};

use job_pool::{builder::PoolBuilder, pool::JobPool};

/// It creates a thread pool with two threads, each with a stack size of 2MB
///
/// Returns:
///
/// A JobPool
fn test_worker() -> JobPool {
    PoolBuilder::with_workers_capacity(1, 2)
        .unwrap()
        .stack_size(2 * 1024 * 1024)
        .build()
}

#[test]
fn test_run_sync() {
    let pool = test_worker();
    let first = pool
        .run_sync_job(|| {
            pub fn sum_from_zero(n: i32) -> i32 {
                (0..n + 1).fold(0, |a, b| a + b)
            }
            std::thread::sleep(Duration::from_millis(120));
            sum_from_zero(12)
        })
        .join()
        .expect("Failed to run the task ");

    assert_eq!(first, 78);
    let second = pool
        .run_sync_job(|| {
            pub fn geometric_progression(a: i32, b: i32, c: i32) -> i32 {
                let mut value = 0;
                for i in 1..c + 1 {
                    value = a * b.pow((i - 1) as u32)
                }
                value
            }
            std::thread::sleep(Duration::from_millis(100));
            geometric_progression(1, 2, 10)
        })
        .join()
        .expect("Failed to run the task ");
    assert_eq!(second, 512);
    let times = vec![
        pool.state.tasks_completion_time.pop().unwrap(),
        pool.state.tasks_completion_time.pop().unwrap(),
    ];
    assert_eq!(times.len(), 2);
    //Calculating average task completion time.
    assert!(times.iter().sum::<u128>() as f32 / times.len() as f32 > 100.0);
}

#[test]
fn test_run_async() {
    let pool = test_worker();
    let task_result = pool
        .run_async_job(async {
            pub fn sum_from_zero(n: i32) -> i32 {
                (0..n + 1).fold(0, |a, b| a + b)
            }
            sum_from_zero(12)
        })
        .join()
        .expect("Failed to run the task ");

    assert_eq!(task_result, 78);
}

#[test]
fn job_failed_due_to_timeout() {
    let pool = test_worker();
    let result = pool
        .run_sync_job(|| thread::sleep(Duration::from_millis(100)))
        .join_timeout(Duration::from_millis(10));
    let status = result.unwrap();
    assert!(status.is_err());
    assert!(status.unwrap_err().has_timeout_occurred());
}

#[test]
fn idle_shutdown_pool() {
    let pool = PoolBuilder::with_workers_capacity(0, 2)
        .unwrap()
        .stack_size(2 * 1024 * 1024)
        .keep_alive(Duration::from_millis(10))
        .build();

    assert_eq!(pool.jobs(), 0, "Job Pool is empty");
    pool.run_sync_job(|| {
        pub fn sum_from_zero(n: i32) -> i32 {
            (0..n + 1).fold(0, |a, b| a + b)
        }
        sum_from_zero(12)
    })
    .join()
    .expect("Failed to run the task ");
    assert_eq!(pool.jobs(), 1, "One Job is pushed to pool");

    thread::sleep(Duration::from_millis(200));
    assert_eq!(
        pool.jobs(),
        0,
        "Workers became idle and terminated after timeout"
    );
}

#[test]
fn deallocated_slow_jobs_on_join_timeout() {
    let pool = test_worker();
    assert_eq!(pool.jobs(), 1);

    let _task = pool.run_sync_job(|| {
        println!("Hello Waiting");
        thread::sleep(Duration::from_millis(500));
        println!("Waiting Done ");
    });
    assert!(!pool.join_timeout(Duration::from_millis(10)));
}
