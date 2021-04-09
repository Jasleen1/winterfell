use super::FOLDING_FACTOR;
use crate::utils::uninit_vector;
use crypto::HashFunction;
use math::field::{BaseElement, FieldElement};
use rayon::prelude::*;

pub const MIN_CONCURRENT_DOMAIN: usize = 256;

pub fn evaluate_batch<E: FieldElement>(polys: &[[E; FOLDING_FACTOR]], x: E) -> Vec<E> {
    let n = polys.len();
    if n <= MIN_CONCURRENT_DOMAIN {
        super::evaluate_batch(polys, x)
    } else {
        let mut result: Vec<E> = uninit_vector(n);
        result
            .par_iter_mut()
            .zip(polys.par_iter())
            .for_each(|(result, poly)| {
                *result = super::eval(poly, x);
            });
        result
    }
}

pub fn interpolate_batch<E: FieldElement + From<BaseElement>>(
    xs: &[[BaseElement; FOLDING_FACTOR]],
    ys: &[[E; FOLDING_FACTOR]],
) -> Vec<[E; FOLDING_FACTOR]> {
    debug_assert!(
        xs.len() == ys.len(),
        "number of X coordinates must be equal to number of Y coordinates"
    );
    let n = xs.len();
    if n <= MIN_CONCURRENT_DOMAIN {
        super::interpolate_batch(xs, ys)
    } else {
        let mut result: Vec<[E; FOLDING_FACTOR]> = uninit_vector(n);
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
}

pub fn transpose<E: FieldElement>(source: &[E], stride: usize) -> Vec<[E; FOLDING_FACTOR]> {
    assert!(
        source.len() % (FOLDING_FACTOR * stride) == 0,
        "vector length must be divisible by {}",
        FOLDING_FACTOR * stride
    );
    if source.len() * FOLDING_FACTOR <= MIN_CONCURRENT_DOMAIN {
        super::transpose(source, stride)
    } else {
        let row_count = source.len() / (FOLDING_FACTOR * stride);
        let mut result = super::to_quartic_vec(super::uninit_vector(row_count * FOLDING_FACTOR));
        result.par_iter_mut().enumerate().for_each(|(i, element)| {
            super::transpose_element(element, &source, i, stride, row_count);
        });
        result
    }
}

pub fn to_quartic_vec<E: FieldElement>(vector: Vec<E>) -> Vec<[E; FOLDING_FACTOR]> {
    // just a convenience function calling single-threaded version of to_quartic_vec
    // since there isn't anything different to do in a multi-threaded version.
    super::to_quartic_vec(vector)
}

pub fn hash_values<E: FieldElement>(
    values: &[[E; FOLDING_FACTOR]],
    hash: HashFunction,
) -> Vec<[u8; 32]> {
    if values.len() <= MIN_CONCURRENT_DOMAIN {
        super::hash_values(values, hash)
    } else {
        let mut result: Vec<[u8; 32]> = uninit_vector(values.len());
        result
            .par_iter_mut()
            .zip(values.par_iter())
            .for_each(|(r, v)| {
                hash(E::elements_as_bytes(v), r);
            });
        result
    }
}
