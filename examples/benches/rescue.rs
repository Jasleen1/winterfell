use common::{FieldExtension, ProofOptions};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use prover::crypto::hash;
use std::time::Duration;
use winterfell::{rescue, Example};

const SIZES: [usize; 2] = [256, 512];

fn rescue(c: &mut Criterion) {
    let mut group = c.benchmark_group("rescue");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(25));

    let options = ProofOptions::new(32, 32, 0, hash::blake3, FieldExtension::None);
    let mut resc = rescue::RescueExample::new(options);
    for &size in SIZES.iter() {
        let assertions = resc.prepare(size);
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &assertions,
            |bench, a| {
                bench.iter(|| resc.prove(a.clone()));
            },
        );
    }
    group.finish();
}

criterion_group!(rescue_group, rescue);
criterion_main!(rescue_group);
