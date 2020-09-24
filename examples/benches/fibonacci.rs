use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use winterfell::fibonacci;

const SIZES: [usize; 3] = [16_384, 65_536, 262_144];

fn fibonacci(c: &mut Criterion) {
    let mut group = c.benchmark_group("fibonacci");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    let fib = fibonacci::get_example();
    for &size in SIZES.iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |bench, &size| {
            bench.iter(|| fib.prove(size, 8, 32));
        });
    }
    group.finish();
}

criterion_group!(fibonacci_group, fibonacci);
criterion_main!(fibonacci_group);
