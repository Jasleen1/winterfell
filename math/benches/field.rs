use criterion::{black_box, criterion_group, criterion_main, Criterion};
use math::field::{FieldElement, StarkField};
use std::convert::TryInto;

pub fn add(c: &mut Criterion) {
    let x = FieldElement::rand();
    let y = FieldElement::rand();
    c.bench_function("field_add", |bench| {
        bench.iter(|| black_box(x) + black_box(y))
    });
}

pub fn sub(c: &mut Criterion) {
    let x = FieldElement::rand();
    let y = FieldElement::rand();
    c.bench_function("field_sub", |bench| {
        bench.iter(|| black_box(x) - black_box(y))
    });
}

pub fn mul(c: &mut Criterion) {
    let x = FieldElement::rand();
    let y = FieldElement::rand();
    c.bench_function("field_mul", |bench| {
        bench.iter(|| black_box(x) * black_box(y))
    });
}

pub fn exp(c: &mut Criterion) {
    let x = FieldElement::rand();
    let y = u128::from_le_bytes(
        FieldElement::rand()
            .to_bytes()
            .as_slice()
            .try_into()
            .unwrap(),
    );
    c.bench_function("field_exp", |bench| {
        bench.iter(|| FieldElement::exp(black_box(x), black_box(y)))
    });
}

pub fn inv(c: &mut Criterion) {
    let x = FieldElement::rand();
    c.bench_function("field_inv", |bench| {
        bench.iter(|| FieldElement::inv(black_box(x)))
    });
}

criterion_group!(field_group, add, sub, mul, exp, inv);
criterion_main!(field_group);
