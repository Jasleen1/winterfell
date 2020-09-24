use criterion::criterion_main;

mod fibonacci;

criterion_main!(fibonacci::group);
