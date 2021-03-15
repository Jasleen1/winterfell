use prover::math::field::{BaseElement, FieldElement};

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

#[cfg(test)]
pub fn build_proof_options(use_extension_field: bool) -> common::ProofOptions {
    use common::{FieldExtension, ProofOptions};
    use prover::crypto::hash;
    let extension = if use_extension_field {
        FieldExtension::Quadratic
    } else {
        FieldExtension::None
    };
    ProofOptions::new(28, 16, 0, hash::blake3, extension)
}
