use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use math::{
    field::{AsBytes, BaseElement, FieldElement},
    utils::batch_inversion,
};
use rand::Rng;
use std::{convert::TryInto, time::Duration};

const SIZES: [usize; 3] = [262_144, 524_288, 1_048_576];

pub fn add(c: &mut Criterion) {
    let x = BaseElement::rand();
    let y = BaseElement::rand();
    c.bench_function("field_add", |bench| {
        bench.iter(|| black_box(x) + black_box(y))
    });
}

pub fn sub(c: &mut Criterion) {
    let x = BaseElement::rand();
    let y = BaseElement::rand();
    c.bench_function("field_sub", |bench| {
        bench.iter(|| black_box(x) - black_box(y))
    });
}

pub fn mul(c: &mut Criterion) {
    let x = BaseElement::rand();
    let y = BaseElement::rand();
    c.bench_function("field_mul", |bench| {
        bench.iter(|| black_box(x) * black_box(y))
    });
}

pub fn exp(c: &mut Criterion) {
    let x = BaseElement::rand();
    let y = u128::from_le_bytes(BaseElement::rand().as_bytes().try_into().unwrap());
    c.bench_function("field_exp", |bench| {
        bench.iter(|| BaseElement::exp(black_box(x), black_box(y)))
    });
}

pub fn inv(c: &mut Criterion) {
    let x = BaseElement::rand();
    c.bench_function("field_inv", |bench| {
        bench.iter(|| BaseElement::inv(black_box(x)))
    });
}

pub fn batch_inv(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_inv");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    for &size in SIZES.iter() {
        let values = BaseElement::prng_vector(get_seed(), size);

        group.bench_function(BenchmarkId::new("no_coeff", size), |bench| {
            bench.iter_with_large_drop(|| batch_inversion(&values));
        });
    }

    group.finish();
}

criterion_group!(field_group, add, sub, mul, exp, inv, batch_inv);
criterion_main!(field_group);

fn get_seed() -> [u8; 32] {
    rand::thread_rng().gen::<[u8; 32]>()
}
