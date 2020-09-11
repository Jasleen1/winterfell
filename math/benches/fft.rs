use common::utils::as_bytes;
use criterion::{criterion_group, BenchmarkId, Criterion};
use math::{fft, field};
use std::time::Duration;

const SIZES: [usize; 3] = [262_144, 524_288, 1_048_576];

fn fft_poly(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_poly");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    for &size in SIZES.iter() {
        let root = field::get_root_of_unity(size);
        let twiddles = fft::get_twiddles(root, size);
        let mut p = field::prng_vector(get_seed(), size);

        group.bench_function(BenchmarkId::new("evaluate", size), |bench| {
            bench.iter(|| fft::evaluate_poly(&mut p, &twiddles, true));
        });
    }

    for &size in SIZES.iter() {
        let root = field::get_root_of_unity(size);
        let twiddles = fft::get_twiddles(root, size);
        let mut p = field::prng_vector(get_seed(), size);

        group.bench_function(BenchmarkId::new("evaluate (permuted)", size), |bench| {
            bench.iter(|| fft::evaluate_poly(&mut p, &twiddles, false));
        });
    }

    group.finish();
}

fn get_twiddles(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_get_twiddles");
    group.sample_size(10);
    for &size in SIZES.iter() {
        let root = field::get_root_of_unity(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |bench, &size| {
            bench.iter(|| fft::get_twiddles(root, size));
        });
    }
    group.finish();
}

criterion_group!(group, fft_poly, get_twiddles);

fn get_seed() -> [u8; 32] {
    let seed = [field::rand(), field::rand()];
    let mut result = [0; 32];
    result.copy_from_slice(as_bytes(&seed));
    result
}
