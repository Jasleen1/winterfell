// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use examples::{fast_fourier_transform, Example};
use std::time::Duration;
use winterfell::{FieldExtension, HashFunction, ProofOptions};

// Use SIZE s.t. Log2(SIZE) + 1 is a power of 2
const SIZES: [usize; 1] = [128];//, 128, 128];

// cargo run -- -b 16 fft -n 8
fn fast_fourier_transform(c: &mut Criterion) {
    let mut group = c.benchmark_group("fast_fourier_transform");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    let options = ProofOptions::new(
        32,
        32,
        0,
        HashFunction::Blake3_256,
        FieldExtension::None,
        4,
        256,
    );

    for &size in SIZES.iter() {
        let fft = fast_fourier_transform::FFTExample::new(size, options.clone());
        group.bench_function(BenchmarkId::from_parameter(size), |bench| {
            bench.iter(|| fft.prove());
        });
    }
    group.finish();
}

criterion_group!(fast_fourier_transform_group, fast_fourier_transform);
criterion_main!(fast_fourier_transform_group);