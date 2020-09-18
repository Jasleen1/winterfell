use criterion::criterion_main;

mod fft;
mod field;

criterion_main!(field::group, fft::group);
