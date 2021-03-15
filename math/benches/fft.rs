use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use math::{
    fft,
    field::{BaseElement, QuadExtension, StarkField},
};
use rand::Rng;
use std::time::Duration;

const SIZES: [usize; 3] = [262_144, 524_288, 1_048_576];

fn fft_poly(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_poly");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    for &size in SIZES.iter() {
        let twiddles = fft::get_twiddles::<BaseElement>(size);
        let mut p = BaseElement::prng_vector(get_seed(), size);

        group.bench_function(BenchmarkId::new("evaluate", size), |bench| {
            bench.iter(|| fft::evaluate_poly(&mut p, &twiddles));
        });
    }

    for &size in SIZES.iter() {
        let twiddles = fft::get_twiddles::<BaseElement>(size);
        let mut p = BaseElement::prng_vector(get_seed(), size)
            .into_iter()
            .map(QuadExtension::from)
            .collect::<Vec<_>>();

        group.bench_function(
            BenchmarkId::new("evaluate (extension field)", size),
            |bench| {
                bench.iter(|| fft::evaluate_poly(&mut p, &twiddles));
            },
        );
    }

    for &size in SIZES.iter() {
        let inv_twiddles = fft::get_inv_twiddles::<BaseElement>(size);
        let mut p = BaseElement::prng_vector(get_seed(), size);

        group.bench_function(BenchmarkId::new("interpolate", size), |bench| {
            bench.iter(|| fft::interpolate_poly(&mut p, &inv_twiddles));
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

criterion_group!(fft_group, fft_poly, get_twiddles);
criterion_main!(fft_group);

fn get_seed() -> [u8; 32] {
    rand::thread_rng().gen::<[u8; 32]>()
}
