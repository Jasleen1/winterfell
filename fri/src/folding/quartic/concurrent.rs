use crate::utils::uninit_vector;
use math::field::{BaseElement, FieldElement};
use rayon::prelude::*;

/// Evaluates a batch of degree 3 polynomials at the provided X coordinate.
pub fn evaluate_batch<E: FieldElement>(polys: &[[E; 4]], x: E) -> Vec<E> {
    let n = polys.len();

    let mut result: Vec<E> = Vec::with_capacity(n);
    unsafe {
        result.set_len(n);
    }

    result
        .par_iter_mut()
        .zip(polys.par_iter())
        .for_each(|(result, poly)| {
            *result = super::eval(poly, x);
        });

    result
}

pub fn interpolate_batch<E: FieldElement + From<BaseElement>>(
    xs: &[[BaseElement; 4]],
    ys: &[[E; 4]],
) -> Vec<[E; 4]> {
    debug_assert!(
        xs.len() == ys.len(),
        "number of X coordinates must be equal to number of Y coordinates"
    );
    let n = xs.len();
    let mut result: Vec<[E; 4]> = uninit_vector(n);

    let num_batches = rayon::current_num_threads().next_power_of_two();
    let batch_size = n / num_batches;

    result
        .par_chunks_mut(batch_size)
        .enumerate()
        .for_each(|(i, batch)| {
            let start = i * batch_size;
            let end = start + batch_size;
            super::interpolate_batch_into(&xs[start..end], &ys[start..end], batch);
        });

    result
}

pub fn transpose<E: FieldElement>(source: &[E], stride: usize) -> Vec<[E; 4]> {
    assert!(
        source.len() % (4 * stride) == 0,
        "vector length must be divisible by {}",
        4 * stride
    );
    let row_count = source.len() / (4 * stride);

    let mut result = super::to_quartic_vec(super::uninit_vector(row_count * 4));
    result.par_iter_mut().enumerate().for_each(|(i, element)| {
        super::transpose_element(element, &source, i, stride, row_count);
    });
    result
}

/// Re-interprets a vector of field elements as a vector of quartic elements.
pub fn to_quartic_vec<E: FieldElement>(vector: Vec<E>) -> Vec<[E; 4]> {
    super::to_quartic_vec(vector)
}
