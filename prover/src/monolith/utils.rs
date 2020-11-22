use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
    polynom,
};

/// Computes a[i] + b[i] for all i and stores the results in a.
pub fn add_in_place<E: FieldElement>(a: &mut [E], b: &[E]) {
    assert!(
        a.len() == b.len(),
        "number of values must be the same for both operands"
    );
    for i in 0..a.len() {
        a[i] = a[i] + b[i];
    }
}

/// Computes a[i] + b[i] * c for all i and saves result into a.
pub fn mul_acc<E: FieldElement>(a: &mut [E], b: &[E], c: E) {
    assert!(
        a.len() == b.len(),
        "number of values must be the same for both slices"
    );
    for i in 0..a.len() {
        a[i] = a[i] + b[i] * c;
    }
}

/// Determines degree of a polynomial implied by the provided evaluations
pub fn infer_degree<E: FieldElement<PositiveInteger = u128> + From<BaseElement>>(
    evaluations: &[E],
) -> usize {
    assert!(
        evaluations.len().is_power_of_two(),
        "number of evaluations must be a power of 2"
    );
    let mut poly = evaluations.to_vec();
    let root = E::from(BaseElement::get_root_of_unity(
        evaluations.len().trailing_zeros(),
    ));
    let inv_twiddles = fft::get_inv_twiddles(root, evaluations.len());
    fft::interpolate_poly(&mut poly, &inv_twiddles, true);
    polynom::degree_of(&poly)
}
