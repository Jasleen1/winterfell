use math::field;

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
