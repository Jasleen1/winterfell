use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use crypto::{hash, merkle, utils::uninit_vector};
use rand::{rngs::ThreadRng, thread_rng, RngCore};

pub fn merkle_tree_construction(c: &mut Criterion) {
    use kompact::prelude::ActorRefFactory;

    let mut merkle_group = c.benchmark_group("Merkle tree construction");

    static BATCH_SIZES: [usize; 5] = [16384, 32768, 65536, 131072, 262144];

    for size in &BATCH_SIZES {
        let mut csprng: ThreadRng = thread_rng();

        let data: Vec<[u8; 32]> = {
            let mut res = uninit_vector(*size);
            for i in 0..*size {
                let mut v = [0u8; 32];
                csprng.fill_bytes(&mut v);
                res[i] = v;
            }
            res
        };

        let mut config = kompact::prelude::KompactConfig::default();
        config.threads(15);
        let system = config.build().expect("system");
        let manager = system.create(move || merkle::concurrent_merkle::Manager::new(15));
        system.start(&manager);
        let manager_ref = manager.actor_ref().hold().expect("live");

        merkle_group.bench_with_input(BenchmarkId::new("sequential", size), &data, |b, i| {
            b.iter(|| merkle::build_merkle_nodes(&i, crate::hash::sha3))
        });
        merkle_group.bench_with_input(BenchmarkId::new("concurrent", size), &data, |b, i| {
            b.iter(|| {
                let work = merkle::concurrent_merkle::Work::with(&i, crate::hash::sha3);
                manager_ref.ask(kompact::prelude::Ask::of(work)).wait();
            })
        });
        system.shutdown().expect("shutdown");
    }
}

criterion_group!(merkle_group, merkle_tree_construction,);
criterion_main!(merkle_group);
