use claim::claim::Claim;
use criterion::*;
use rand::prelude::*;
use ritelinked::LinkedHashMap;
use secp256k1::Secp256k1;

pub fn generate_10000_claims() -> LinkedHashMap<String, Claim> {
    let mut claims: LinkedHashMap<String, Claim> = LinkedHashMap::new();
    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();

    (0..10000).for_each(|_| {
        let (_, public_key) = secp.generate_keypair(&mut rng);
        let claim = Claim::new(public_key.to_string(), 1);
        claims.insert(public_key.to_string(), claim);
    });

    claims
}

pub fn generate_100000_claims() -> LinkedHashMap<String, Claim> {
    let mut claims: LinkedHashMap<String, Claim> = LinkedHashMap::new();
    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();

    (0..100000).for_each(|_| {
        let (_, public_key) = secp.generate_keypair(&mut rng);
        let claim = Claim::new(public_key.to_string(), 1);
        claims.insert(public_key.to_string(), claim);
    });

    claims
}

pub fn check_collision(pointers: Vec<(String, u128)>) -> bool {
    pointers.len() > 1
}

fn calculate_pointers(claim_map: LinkedHashMap<String, Claim>, nonce: u128) {
    let mut pointers = claim_map
        .iter()
        .map(|(pk, claim)| return (pk.clone(), claim.get_pointer(nonce)))
        .collect::<Vec<_>>();
    pointers.retain(|(_, v)| !v.is_none());
    let mut raw_pointers = pointers
        .iter()
        .map(|(k, v)| {
            return (k.clone(), v.unwrap());
        })
        .collect::<Vec<_>>();
    let min = raw_pointers.iter().min_by_key(|(_, v)| v).unwrap().1;
    raw_pointers.retain(|(_, v)| *v == min);
}

fn pointers_bench_10000(c: &mut Criterion) {
    let claims = generate_10000_claims();
    let mut rng = rand::thread_rng();

    c.bench_function("pointers_10k", move |b| {
        b.iter_batched(
            || claims.clone(),
            |claims| calculate_pointers(claims, rng.gen()),
            BatchSize::SmallInput,
        )
    });
}

fn pointers_bench_100000(c: &mut Criterion) {
    let claims = generate_100000_claims();
    let mut rng = rand::thread_rng();

    c.bench_function("pointers_100k", move |b| {
        b.iter_batched(
            || claims.clone(),
            |claims| calculate_pointers(claims, rng.gen()),
            BatchSize::SmallInput,
        )
    });
}


criterion_group!(benches, pointers_bench_10000, pointers_bench_100000);
criterion_main!(benches);
