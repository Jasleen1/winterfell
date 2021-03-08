use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use fri::folding::quartic::{self, to_quartic_vec};
use math::field::{BaseElement, FieldElement, StarkField};

static BATCH_SIZES: [usize; 3] = [65536, 131072, 262144];

pub fn interpolate_batch(c: &mut Criterion) {
    let mut interpolate_group = c.benchmark_group("quartic interpolate batch");

    for &size in &BATCH_SIZES {
        let (xs, ys) = build_coordinate_batches(size);
        interpolate_group.bench_function(BenchmarkId::new("sequential", size), |b| {
            b.iter(|| quartic::interpolate_batch(&xs, &ys))
        });

        interpolate_group.bench_function(BenchmarkId::new("concurrent", size), |b| {
            b.iter(|| quartic::concurrent::interpolate_batch(&xs, &ys))
        });
    }
}

criterion_group!(quartic_group, interpolate_batch);
criterion_main!(quartic_group);

// HELPER FUNCTIONS
// ================================================================================================

fn build_coordinate_batches(batch_size: usize) -> (Vec<[BaseElement; 4]>, Vec<[BaseElement; 4]>) {
    let r = BaseElement::get_root_of_unity(batch_size.trailing_zeros());
    let xs = to_quartic_vec(BaseElement::get_power_series(r, batch_size));
    let ys = to_quartic_vec(BaseElement::prng_vector([1; 32], batch_size));
    (xs, ys)
}
