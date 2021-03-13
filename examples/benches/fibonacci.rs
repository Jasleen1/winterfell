use common::{FieldExtension, ProofOptions};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use prover::crypto::hash;
use std::time::Duration;
use winterfell::{fibonacci, Example};

const SIZES: [usize; 3] = [16_384, 65_536, 262_144];

fn fibonacci(c: &mut Criterion) {
    let mut group = c.benchmark_group("fibonacci");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    let options = ProofOptions::new(32, 8, 0, hash::blake3, FieldExtension::None);
    let mut fib = fibonacci::fib2::FibExample::new(options);
    for &size in SIZES.iter() {
        let assertions = fib.prepare(size);
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &assertions,
            |bench, a| {
                bench.iter(|| fib.prove(a.clone()));
            },
        );
    }
    group.finish();
}

criterion_group!(fibonacci_group, fibonacci);
criterion_main!(fibonacci_group);
