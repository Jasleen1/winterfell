use common::ProofOptions;
use prover::{
    crypto::hash::blake3,
    math::field::{BaseElement, FieldElement},
};

pub fn compute_fib_term(n: usize) -> BaseElement {
    let mut t0 = BaseElement::ONE;
    let mut t1 = BaseElement::ONE;

    for _ in 0..(n - 1) {
        t1 = t0 + t1;
        std::mem::swap(&mut t0, &mut t1);
    }

    t1
}

pub fn compute_mulfib_term(n: usize) -> BaseElement {
    let mut t0 = BaseElement::ONE;
    let mut t1 = BaseElement::new(2);

    for _ in 0..(n - 1) {
        t1 = t0 * t1;
        std::mem::swap(&mut t0, &mut t1);
    }

    t1
}

pub fn prepare_options(
    mut blowup_factor: usize,
    mut num_queries: usize,
    grinding_factor: u32,
) -> ProofOptions {
    if blowup_factor == 0 {
        blowup_factor = 16;
    }
    if num_queries == 0 {
        num_queries = 28;
    }

    ProofOptions::new(num_queries, blowup_factor, grinding_factor, blake3)
}
