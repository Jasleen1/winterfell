use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use math::{
    fft,
    field::{BaseElement, FieldElement, QuadElement, StarkField},
};
use rand::Rng;
use std::time::Duration;

const SIZES: [usize; 3] = [262_144, 524_288, 1_048_576];

fn fft_evaluate_poly(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_evaluate_poly");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    let blowup_factor = 8;

    for &size in SIZES.iter() {
        let p = BaseElement::prng_vector(get_seed(), size / blowup_factor);
        let twiddles = fft::get_twiddles::<BaseElement>(size);
        group.bench_function(BenchmarkId::new("simple", size), |bench| {
            bench.iter_with_large_drop(|| {
                let mut result = vec![BaseElement::ZERO; size];
                result[..p.len()].copy_from_slice(&p);
                fft::evaluate_poly(&mut result, &twiddles);
                result
            });
        });
    }

    for &size in SIZES.iter() {
        let p = BaseElement::prng_vector(get_seed(), size / blowup_factor);
        let twiddles = fft::get_twiddles::<BaseElement>(size / blowup_factor);
        group.bench_function(BenchmarkId::new("with_offset", size), |bench| {
            bench.iter_with_large_drop(|| {
                let result = fft::evaluate_poly_with_offset(
                    &p,
                    &twiddles,
                    BaseElement::GENERATOR,
                    blowup_factor,
                );
                result
            });
        });
    }

    for &size in SIZES.iter() {
        let twiddles = fft::get_twiddles::<BaseElement>(size);
        let p = BaseElement::prng_vector(get_seed(), size / blowup_factor)
            .into_iter()
            .map(QuadElement::from)
            .collect::<Vec<_>>();
        group.bench_function(BenchmarkId::new("extension", size), |bench| {
            bench.iter_with_large_drop(|| {
                let mut result = vec![QuadElement::ZERO; size];
                result[..p.len()].copy_from_slice(&p);
                fft::evaluate_poly(&mut result, &twiddles);
                result
            });
        });
    }

    group.finish();
}

fn fft_interpolate_poly(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_interpolate_poly");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    for &size in SIZES.iter() {
        let inv_twiddles = fft::get_inv_twiddles::<BaseElement>(size);
        let p = BaseElement::prng_vector(get_seed(), size);
        group.bench_function(BenchmarkId::new("simple", size), |bench| {
            bench.iter_batched_ref(
                || p.clone(),
                |mut p| fft::interpolate_poly(&mut p, &inv_twiddles),
                BatchSize::LargeInput,
            );
        });
    }

    for &size in SIZES.iter() {
        let inv_twiddles = fft::get_inv_twiddles::<BaseElement>(size);
        let p = BaseElement::prng_vector(get_seed(), size);
        group.bench_function(BenchmarkId::new("with_offset", size), |bench| {
            bench.iter_batched_ref(
                || p.clone(),
                |mut p| {
                    fft::interpolate_poly_with_offset(&mut p, &inv_twiddles, BaseElement::GENERATOR)
                },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

fn get_twiddles(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_get_twiddles");
    group.sample_size(10);
    for &size in SIZES.iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |bench, &size| {
            bench.iter(|| fft::get_twiddles::<BaseElement>(size));
        });
    }
    group.finish();
}

criterion_group!(
    fft_group,
    fft_evaluate_poly,
    fft_interpolate_poly,
    get_twiddles
);
criterion_main!(fft_group);

fn get_seed() -> [u8; 32] {
    rand::thread_rng().gen::<[u8; 32]>()
}
