use prover::math::field::{FieldElement, FieldElementTrait};

pub fn compute_fib_term(n: usize) -> FieldElement {
    let mut t0 = FieldElement::ONE;
    let mut t1 = FieldElement::ONE;

    for _ in 0..(n - 1) {
        t1 = t0 + t1;
        std::mem::swap(&mut t0, &mut t1);
    }

    t1
}

pub fn compute_mulfib_term(n: usize) -> FieldElement {
    let mut t0 = FieldElement::ONE;
    let mut t1 = FieldElement::new(2);

    for _ in 0..(n - 1) {
        t1 = t0 * t1;
        std::mem::swap(&mut t0, &mut t1);
    }

    t1
}
