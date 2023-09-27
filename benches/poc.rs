use rand::Rng;
use sha2::{Digest, Sha256};
use ethereum_types::U256;
use tokio::sync::mpsc::{Sender, channel};
use tokio::task::{spawn};
use criterion::{
    black_box,
    criterion_main,
    criterion_group,
    Criterion,
    async_executor::FuturesExecutor
};

fn generate_sha256() -> U256  {
    let mut rng = rand::thread_rng();
    let data: Vec<u8> = (0..64).map(|_| rng.gen()).collect();
    U256::from_big_endian(&Sha256::digest(data))
}

fn xor_hash(seed: &u64, hash: &U256) -> U256 {
    let mut res: [u64; 4] = [0u64; 4];
    hash.0.iter().enumerate().for_each(|(idx, inner)| {
        res[idx] = inner ^ seed
    });

    U256(res)
}

fn xor_claim_hashes(
    seed: &u64, claim_hashes: &[U256]
) -> Vec<U256> {
    claim_hashes.iter().map(|hash| {
        xor_hash(seed, hash)
    }).collect()
}

async fn concurrent_xor_claim_hashes(
    seed: u64,
    claim_hashes: Vec<U256>,
    tx: Sender<U256>,
) {
    claim_hashes.iter().map(|h| {
        let hash = *h;
        let inner_seed = seed;
        let sender = tx.clone();
        spawn(async move {
            let res = xor_hash(&inner_seed, &hash);
            let _ = sender.send(res).await;
        });
    });
}

fn iter_setup(n: usize) -> (u64, Vec<U256>) {
    let mut rng = rand::thread_rng();
    let seed: u64 = rng.gen();
    let claim_hashes = (0..n).map(|_| generate_sha256()).collect();
    (seed, claim_hashes)
}

fn concurrent_setup(n: usize) -> (Sender<U256>, u64, Vec<U256>) {
    let mut rng = rand::thread_rng();
    let seed: u64 = rng.gen();
    let hashes = (0..n).map(|_| generate_sha256()).collect();
    let (tx, _) = channel(n);
    (tx, seed, hashes)
}

pub fn ten_iter_benchmark(c: &mut Criterion) {
    let (seed, claim_hashes) = iter_setup(10);
    c.bench_function("ten_iter_poc", |b| b.iter(|| {
        xor_claim_hashes(
            black_box(&seed), 
            black_box(&claim_hashes)
        )
    }));
}


pub fn hundred_iter_benchmark(c: &mut Criterion) {
    let (seed, claim_hashes) = iter_setup(100);
    c.bench_function("hundred_iter_poc", |b| b.iter(|| {
        xor_claim_hashes(
            black_box(&seed), 
            black_box(&claim_hashes)
        )
    }));
}


pub fn thousand_iter_benchmark(c: &mut Criterion) {
    let (seed, claim_hashes) = iter_setup(1_000);
    c.bench_function("thousand_iter_poc", |b| b.iter(|| {
        xor_claim_hashes(
            black_box(&seed), 
            black_box(&claim_hashes)
        )
    }));
}

pub fn ten_thousand_iter_benchmark(c: &mut Criterion) {
    let (seed, claim_hashes) = iter_setup(10_000);
    c.bench_function("ten_thousand_iter_poc", |b| b.iter(|| {
        xor_claim_hashes(
            black_box(&seed), 
            black_box(&claim_hashes)
        )
    }));
}

pub fn hundred_thousand_iter_benchmark(c: &mut Criterion) {
    let (seed, claim_hashes) = iter_setup(100_000);
    c.bench_function("hundred_thousand_iter_poc", |b| b.iter(|| {
        xor_claim_hashes(
            black_box(&seed), 
            black_box(&claim_hashes)
        )
    }));
}

pub fn million_iter_benchmark(c: &mut Criterion) {
    let (seed, claim_hashes) = iter_setup(1_000_000);
    c.bench_function("million_iter_poc", |b| b.iter(|| {
        xor_claim_hashes(
            black_box(&seed), 
            black_box(&claim_hashes)
        )
    }));
}

pub fn ten_concurrent_benchmark(c: &mut Criterion) {
    let (tx, seed, claim_hashes) = concurrent_setup(10);
    c.bench_function("ten_concurrent_poc", |b| {
        b.to_async(FuturesExecutor).iter(|| async {
            concurrent_xor_claim_hashes(
                black_box(seed), 
                black_box(claim_hashes.clone()), 
                black_box(tx.clone())
            )
        });
    });
}

pub fn hundred_concurrent_benchmark(c: &mut Criterion) {
    let (tx, seed, claim_hashes) = concurrent_setup(100);
    c.bench_function("hundred_concurrent_poc", |b| {
        b.to_async(FuturesExecutor).iter(|| async {
            concurrent_xor_claim_hashes(
                black_box(seed), 
                black_box(claim_hashes.clone()), 
                black_box(tx.clone())
            )
        });
    });
}

pub fn thousand_concurrent_benchmark(c: &mut Criterion) {
    let (tx, seed, claim_hashes) = concurrent_setup(1_000);
    c.bench_function("thousand_concurrent_poc", |b| {
        b.to_async(FuturesExecutor).iter(|| async {
            concurrent_xor_claim_hashes(
                black_box(seed), 
                black_box(claim_hashes.clone()), 
                black_box(tx.clone())
            )
        });
    });
}

pub fn ten_thousand_concurrent_benchmark(c: &mut Criterion) {
    let (tx, seed, claim_hashes) = concurrent_setup(10_000);
    c.bench_function("ten_thousand_concurrent_poc", |b| {
        b.to_async(FuturesExecutor).iter(|| async {
            concurrent_xor_claim_hashes(
                black_box(seed), 
                black_box(claim_hashes.clone()), 
                black_box(tx.clone())
            )
        });
    });
}

pub fn hundred_thousand_concurrent_benchmark(c: &mut Criterion) {
    let (tx, seed, claim_hashes) = concurrent_setup(100_000);
    c.bench_function("hundred_thousand_concurrent_poc", |b| {
        b.to_async(FuturesExecutor).iter(|| async {
            concurrent_xor_claim_hashes(
                black_box(seed), 
                black_box(claim_hashes.clone()), 
                black_box(tx.clone())
            )
        });
    });
}

pub fn million_concurrent_benchmark(c: &mut Criterion) {
    let (tx, seed, claim_hashes) = concurrent_setup(1_000_000);
    c.bench_function("million_thousand_concurrent_poc", |b| {
        b.to_async(FuturesExecutor).iter(|| async {
            concurrent_xor_claim_hashes(
                black_box(seed), 
                black_box(claim_hashes.clone()), 
                black_box(tx.clone())
            )
        });
    });
}

criterion_group!(
    benches,
    ten_iter_benchmark,
    hundred_iter_benchmark,
    thousand_iter_benchmark,
    ten_thousand_iter_benchmark,
    hundred_thousand_iter_benchmark,
    million_iter_benchmark,
    ten_concurrent_benchmark,
    hundred_concurrent_benchmark,
    thousand_concurrent_benchmark,
    ten_thousand_concurrent_benchmark,
    hundred_thousand_concurrent_benchmark,
    million_concurrent_benchmark
);

criterion_main!(benches);
