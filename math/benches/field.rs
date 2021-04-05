use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use math::{
    field::{f128, f62, AsBytes, FieldElement},
    utils::batch_inversion,
};
use rand::Rng;
use std::{convert::TryInto, time::Duration};

const SIZES: [usize; 3] = [262_144, 524_288, 1_048_576];

pub fn f128_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("f128");

    group.bench_function("add", |bench| {
        let x = f128::BaseElement::rand();
        let y = f128::BaseElement::rand();
        bench.iter(|| black_box(x) + black_box(y))
    });

    group.bench_function("sub", |bench| {
        let x = f128::BaseElement::rand();
        let y = f128::BaseElement::rand();
        bench.iter(|| black_box(x) - black_box(y))
    });

    group.bench_function("mul", |bench| {
        let x = f128::BaseElement::rand();
        let y = f128::BaseElement::rand();
        bench.iter(|| black_box(x) * black_box(y))
    });

    group.bench_function("exp", |bench| {
        let x = f128::BaseElement::rand();
        let y = u128::from_le_bytes(f128::BaseElement::rand().as_bytes().try_into().unwrap());
        bench.iter(|| f128::BaseElement::exp(black_box(x), black_box(y)))
    });

    group.bench_function("inv", |bench| {
        let x = f128::BaseElement::rand();
        bench.iter(|| f128::BaseElement::inv(black_box(x)))
    });
}

pub fn f62_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("f62");

    group.bench_function("add", |bench| {
        let x = f62::BaseElement::rand();
        let y = f62::BaseElement::rand();
        bench.iter(|| black_box(x) + black_box(y))
    });

    group.bench_function("sub", |bench| {
        let x = f62::BaseElement::rand();
        let y = f62::BaseElement::rand();
        bench.iter(|| black_box(x) - black_box(y))
    });

    group.bench_function("mul", |bench| {
        let x = f62::BaseElement::rand();
        let y = f62::BaseElement::rand();
        bench.iter(|| black_box(x) * black_box(y))
    });

    group.bench_function("exp", |bench| {
        let x = f62::BaseElement::rand();
        let y = u64::from_le_bytes(f62::BaseElement::rand().as_bytes().try_into().unwrap());
        bench.iter(|| f62::BaseElement::exp(black_box(x), black_box(y)))
    });

    group.bench_function("inv", |bench| {
        let x = f62::BaseElement::rand();
        bench.iter(|| f62::BaseElement::inv(black_box(x)))
    });
}

pub fn batch_inv(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_inv");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    for &size in SIZES.iter() {
        let values = f128::BaseElement::prng_vector(get_seed(), size);

        group.bench_function(BenchmarkId::new("no_coeff", size), |bench| {
            bench.iter_with_large_drop(|| batch_inversion(&values));
        });
    }

    group.finish();
}

criterion_group!(field_group, batch_inv, f128_ops, f62_ops);
criterion_main!(field_group);

fn get_seed() -> [u8; 32] {
    rand::thread_rng().gen::<[u8; 32]>()
}
