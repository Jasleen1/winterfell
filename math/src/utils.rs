use crate::field::FieldElement;

#[cfg(feature = "concurrent")]
use rayon::prelude::*;

// MATH FUNCTIONS
// ================================================================================================

/// Computes a[i] + b[i] for all i and stores the results in a.
pub fn add_in_place<E: FieldElement>(a: &mut [E], b: &[E]) {
    assert!(
        a.len() == b.len(),
        "number of values must be the same for both operands"
    );

    #[cfg(not(feature = "concurrent"))]
    a.iter_mut().zip(b).for_each(|(a, &b)| *a += b);

    #[cfg(feature = "concurrent")]
    a.par_iter_mut()
        .zip(b.par_iter())
        .for_each(|(a, &b)| *a += b);
}

/// Computes a[i] + b[i] * c for all i and saves result into a.
pub fn mul_acc<B, E>(a: &mut [E], b: &[B], c: E)
where
    B: FieldElement,
    E: FieldElement + From<B>,
{
    assert!(
        a.len() == b.len(),
        "number of values must be the same for both slices"
    );

    #[cfg(not(feature = "concurrent"))]
    a.iter_mut().zip(b).for_each(|(a, &b)| {
        *a += E::from(b) * c;
    });

    #[cfg(feature = "concurrent")]
    a.par_iter_mut().zip(b).for_each(|(a, &b)| {
        *a += E::from(b) * c;
    });
}

/// Returns base 2 logarithm of `n`, where `n` is a power of two.
pub fn log2(n: usize) -> u32 {
    assert!(n.is_power_of_two(), "n must be a power of two");
    n.trailing_zeros()
}

// VECTOR FUNCTIONS
// ================================================================================================

pub fn uninit_vector<T>(length: usize) -> Vec<T> {
    let mut vector = Vec::with_capacity(length);
    unsafe {
        vector.set_len(length);
    }
    vector
}

pub fn remove_leading_zeros<E: FieldElement>(values: &[E]) -> Vec<E> {
    for i in (0..values.len()).rev() {
        if values[i] != E::ZERO {
            return values[..(i + 1)].to_vec();
        }
    }

    [].to_vec()
}
