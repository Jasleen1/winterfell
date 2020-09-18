use math::{fft, field, polynom};

/// Computes a[i] + b[i] for all i and stores the results in a.
pub fn add_in_place(a: &mut [u128], b: &[u128]) {
    assert!(
        a.len() == b.len(),
        "number of values must be the same for both operands"
    );
    for i in 0..a.len() {
        a[i] = field::add(a[i], b[i]);
    }
}

/// Computes a[i] + b[i] * c for all i and saves result into a.
pub fn mul_acc(a: &mut [u128], b: &[u128], c: u128) {
    assert!(
        a.len() == b.len(),
        "number of values must be the same for both slices"
    );
    for i in 0..a.len() {
        a[i] = field::add(a[i], field::mul(b[i], c));
    }
}

/// Determines degree of a polynomial implied by the provided evaluations
pub fn infer_degree(evaluations: &[u128]) -> usize {
    assert!(
        evaluations.len().is_power_of_two(),
        "number of evaluations must be a power of 2"
    );
    let mut poly = evaluations.to_vec();
    let root = field::get_root_of_unity(evaluations.len());
    let inv_twiddles = fft::get_inv_twiddles(root, evaluations.len());
    fft::interpolate_poly(&mut poly, &inv_twiddles, true);
    polynom::degree_of(&poly)
}
