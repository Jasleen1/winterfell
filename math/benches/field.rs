use criterion::{black_box, criterion_group, Criterion};
use math::field;

pub fn add(c: &mut Criterion) {
    let x = field::rand();
    let y = field::rand();
    c.bench_function("add", |bench| {
        bench.iter(|| field::add(black_box(x), black_box(y)))
    });
}

pub fn mul(c: &mut Criterion) {
    let x = field::rand();
    let y = field::rand();
    c.bench_function("mul", |bench| {
        bench.iter(|| field::mul(black_box(x), black_box(y)))
    });
}

pub fn exp(c: &mut Criterion) {
    let x = field::rand();
    let y = field::rand();
    c.bench_function("exp", |bench| {
        bench.iter(|| field::exp(black_box(x), black_box(y)))
    });
}

pub fn inv(c: &mut Criterion) {
    let x = field::rand();
    c.bench_function("inv", |bench| bench.iter(|| field::inv(black_box(x))));
}

criterion_group!(group, add, mul, exp, inv);
