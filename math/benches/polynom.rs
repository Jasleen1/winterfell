use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
    polynom,
};
use rand::Rng;
use std::time::Duration;

const SIZES: [usize; 3] = [262_144, 524_288, 1_048_576];

fn syn_div(c: &mut Criterion) {
    let mut group = c.benchmark_group("syn_div");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    for &size in SIZES.iter() {
        let stride = 8;
        let mut values = BaseElement::prng_vector(get_seed(), size);
        for v in values.iter_mut().skip(stride) {
            *v = BaseElement::ZERO;
        }
        let inv_twiddles = fft::get_inv_twiddles::<BaseElement>(size);
        fft::interpolate_poly(&mut values, &inv_twiddles);
        let p = values;
        let z_power = size / stride;

        group.bench_function(BenchmarkId::new("high_degree", size), |bench| {
            bench.iter_batched_ref(
                || p.clone(),
                |mut p| polynom::syn_div(&mut p, z_power, BaseElement::ONE),
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

criterion_group!(polynom_group, syn_div);
criterion_main!(polynom_group);

fn get_seed() -> [u8; 32] {
    rand::thread_rng().gen::<[u8; 32]>()
}
