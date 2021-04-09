use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use fri::{DefaultProverChannel, FriOptions, FriProver};
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
    utils::{get_power_series, log2},
};
use std::time::Duration;

static BATCH_SIZES: [usize; 3] = [65536, 131072, 262144];
static BLOWUP_FACTOR: usize = 8;
static DOMAIN_OFFSET: BaseElement = BaseElement::GENERATOR;

pub fn build_layers(c: &mut Criterion) {
    let mut fri_group = c.benchmark_group("FRI prover");
    fri_group.sample_size(10);
    fri_group.measurement_time(Duration::from_secs(10));

    let options = FriOptions::new(BLOWUP_FACTOR, DOMAIN_OFFSET, crypto::hash::blake3);

    for &domain_size in &BATCH_SIZES {
        let g = BaseElement::get_root_of_unity(log2(domain_size));
        let domain = get_power_series(g, domain_size);
        let evaluations = build_evaluations(domain_size);

        fri_group.bench_with_input(
            BenchmarkId::new("build_layers", domain_size),
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

fn build_evaluations(domain_size: usize) -> Vec<BaseElement> {
    let mut p = BaseElement::prng_vector([1; 32], domain_size / BLOWUP_FACTOR);
    p.resize(domain_size, BaseElement::ZERO);
    let twiddles = fft::get_twiddles::<BaseElement>(domain_size);
    fft::evaluate_poly(&mut p, &twiddles);
    p
}
