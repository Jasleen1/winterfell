use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use math::{
    fft,
    field::{BaseElement, StarkField},
};
use rand::Rng;
use std::time::Duration;

const SIZES: [usize; 3] = [262_144, 524_288, 1_048_576];

fn fft_poly(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_poly");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    for &size in SIZES.iter() {
        let root = BaseElement::get_root_of_unity(size.trailing_zeros());
        let twiddles = fft::get_twiddles(root, size);
        let mut p = BaseElement::prng_vector(get_seed(), size);

        group.bench_function(BenchmarkId::new("evaluate", size), |bench| {
            bench.iter(|| fft::evaluate_poly(&mut p, &twiddles));
        });
    }

    group.finish();
}

fn get_twiddles(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_get_twiddles");
    group.sample_size(10);
    for &size in SIZES.iter() {
        let root = BaseElement::get_root_of_unity(size.trailing_zeros());
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |bench, &size| {
            bench.iter(|| fft::get_twiddles(root, size));
        });
    }
    group.finish();
}

criterion_group!(fft_group, fft_poly, get_twiddles);
criterion_main!(fft_group);

fn get_seed() -> [u8; 32] {
    rand::thread_rng().gen::<[u8; 32]>()
}
