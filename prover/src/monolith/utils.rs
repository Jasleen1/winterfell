use math::{
    fft,
    field::{FieldElement, StarkField},
    polynom,
};

/// Computes a[i] + b[i] for all i and stores the results in a.
pub fn add_in_place(a: &mut [FieldElement], b: &[FieldElement]) {
    assert!(
        a.len() == b.len(),
        "number of values must be the same for both operands"
    );
    for i in 0..a.len() {
        a[i] = a[i] + b[i];
    }
}

/// Computes a[i] + b[i] * c for all i and saves result into a.
pub fn mul_acc(a: &mut [FieldElement], b: &[FieldElement], c: FieldElement) {
    assert!(
        a.len() == b.len(),
        "number of values must be the same for both slices"
    );
    for i in 0..a.len() {
        a[i] = a[i] + b[i] * c;
    }
}

/// Determines degree of a polynomial implied by the provided evaluations
pub fn infer_degree(evaluations: &[FieldElement]) -> usize {
    assert!(
        evaluations.len().is_power_of_two(),
        "number of evaluations must be a power of 2"
    );
    let mut poly = evaluations.to_vec();
    let root = FieldElement::get_root_of_unity(evaluations.len().trailing_zeros());
    let inv_twiddles = fft::get_inv_twiddles(root, evaluations.len());
    fft::interpolate_poly(&mut poly, &inv_twiddles, true);
    polynom::degree_of(&poly)
}
