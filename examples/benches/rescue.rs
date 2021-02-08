use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use winterfell::rescue;

const SIZES: [usize; 2] = [256, 512];

fn rescue(c: &mut Criterion) {
    let mut group = c.benchmark_group("rescue");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(25));

    let mut resc = rescue::get_example();
    for &size in SIZES.iter() {
        let assertions = resc.prepare(size, 32, 32, 0);
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &assertions,
            |bench, a| {
                bench.iter(|| resc.prove(&a));
            },
        );
    }
    group.finish();
}

criterion_group!(rescue_group, rescue);
criterion_main!(rescue_group);
