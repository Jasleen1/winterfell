use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use fri::{DefaultProverChannel, FriOptions, ConcurrentProver as FriProver};
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
};
use std::time::Duration;

static BATCH_SIZES: [usize; 3] = [65536, 131072, 262144];
static BLOWUP_FACTOR: usize = 8;

pub fn build_layers(c: &mut Criterion) {
    let mut fri_group = c.benchmark_group("FRI prover");
    fri_group.sample_size(10);
    fri_group.measurement_time(Duration::from_secs(10));

    let options = FriOptions::new(BLOWUP_FACTOR, crypto::hash::blake3);

    for &domain_size in &BATCH_SIZES {
        let g = BaseElement::get_root_of_unity(domain_size.trailing_zeros());
        let domain = BaseElement::get_power_series(g, domain_size);
        let evaluations = build_evaluations(g, domain_size);

        fri_group.bench_with_input(
            BenchmarkId::new("build_layers (concurrent)", domain_size),
            &evaluations,
            |b, e| {
                let mut prover = FriProver::new(options.clone());
                b.iter_batched(
                    || e.clone(),
                    |evaluations| {
                        let mut channel =
                            DefaultProverChannel::new(options.clone(), domain_size, 32);
                        prover.build_layers(&mut channel, evaluations, &domain);
                        prover.reset();
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }
}

criterion_group!(fri_prover_group, build_layers);
criterion_main!(fri_prover_group);

// HELPER FUNCTIONS
// ================================================================================================

fn build_evaluations(g: BaseElement, domain_size: usize) -> Vec<BaseElement> {
    let mut p = BaseElement::prng_vector([1; 32], domain_size / BLOWUP_FACTOR);
    p.resize(domain_size, BaseElement::ZERO);
    let twiddles = fft::get_twiddles(g, domain_size);
    fft::evaluate_poly(&mut p, &twiddles, true);
    p
}
