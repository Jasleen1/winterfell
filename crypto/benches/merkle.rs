use criterion::{criterion_group, criterion_main, Criterion};
use crypto::{hash, merkle, utils::uninit_vector};
use rand::{rngs::ThreadRng, thread_rng, RngCore};

pub fn sequential_tree_construction(c: &mut Criterion) {
    static BATCH_SIZES: [usize; 3] = [32768, 65536, 131072];

    c.bench_function_over_inputs(
        "sequential build_merkle_nodes",
        |b, &&size| {
            let mut csprng: ThreadRng = thread_rng();

            let data: Vec<[u8; 32]> = {
                let mut res = uninit_vector(size);
                for i in 0..size {
                    let mut v = [0u8; 32];
                    csprng.fill_bytes(&mut v);
                    res[i] = v;
                }
                res
            };

            b.iter(|| merkle::build_merkle_nodes(&data, crate::hash::sha3));
        },
        &BATCH_SIZES,
    );
}

pub fn concurrent_tree_construction(c: &mut Criterion) {
    use kompact::prelude::ActorRefFactory;

    static BATCH_SIZES: [usize; 3] = [32768, 65536, 131072];

    c.bench_function_over_inputs(
        "concurrent build_merkle_nodes",
        |b, &&size| {
            let mut csprng: ThreadRng = thread_rng();

            let data: Vec<[u8; 32]> = {
                let mut res = uninit_vector(size);
                for i in 0..size {
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

            b.iter(|| {
                let work = merkle::concurrent_merkle::Work::with(&data, crate::hash::sha3);
                manager_ref.ask(kompact::prelude::Ask::of(work)).wait();
            });
            system.shutdown().expect("shutdown");
        },
        &BATCH_SIZES,
    );
}

criterion_group!(
    merkle_group,
    sequential_tree_construction,
    concurrent_tree_construction,
);
criterion_main!(merkle_group);
