use crate::field::{FieldElement, StarkField};

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================
const USIZE_BITS: usize = 0_usize.count_zeros() as usize;
const MAX_LOOP: usize = 256;

// POLYNOMIAL EVALUATION AND INTERPOLATION
// ================================================================================================

/// Evaluates polynomial `p` using FFT algorithm; the evaluation is done in-place, meaning
/// `p` is updated with results of the evaluation.
///
/// If `unpermute` parameter is set to false, the evaluations will be left in permuted state.
pub fn evaluate_poly(p: &mut [FieldElement], twiddles: &[FieldElement], unpermute: bool) {
    debug_assert!(p.len() == twiddles.len() * 2, "Invalid number of twiddles");
    fft_in_place(p, &twiddles, 1, 1, 0);
    if unpermute {
        permute(p);
    }
}

/// Uses FFT algorithm to interpolate a polynomial from provided values `v`; the interpolation
/// is done in-place, meaning `v` is updated with polynomial coefficients.
///
/// If `unpermute` parameter is set to false, the evaluations will be left in permuted state.
pub fn interpolate_poly(v: &mut [FieldElement], inv_twiddles: &[FieldElement], unpermute: bool) {
    fft_in_place(v, &inv_twiddles, 1, 1, 0);
    let inv_length = FieldElement::inv((v.len() as u64).into());
    for e in v.iter_mut() {
        *e = *e * inv_length;
    }
    if unpermute {
        permute(v);
    }
}

// CORE FFT ALGORITHM
// ================================================================================================

/// In-place recursive FFT with permuted output.
/// Adapted from: https://github.com/0xProject/OpenZKP/tree/master/algebra/primefield/src/fft
pub fn fft_in_place(
    values: &mut [FieldElement],
    twiddles: &[FieldElement],
    count: usize,
    stride: usize,
    offset: usize,
) {
    let size = values.len() / stride;
    debug_assert!(size.is_power_of_two());
    debug_assert!(offset < stride);
    debug_assert_eq!(values.len() % size, 0);

    // Keep recursing until size is 2
    if size > 2 {
        if stride == count && count < MAX_LOOP {
            fft_in_place(values, twiddles, 2 * count, 2 * stride, offset);
        } else {
            fft_in_place(values, twiddles, count, 2 * stride, offset);
            fft_in_place(values, twiddles, count, 2 * stride, offset + stride);
        }
    }

    for offset in offset..(offset + count) {
        butterfly(values, offset, stride);
    }

    let last_offset = offset + size * stride;
    for (i, offset) in (offset..last_offset)
        .step_by(2 * stride)
        .enumerate()
        .skip(1)
    {
        for j in offset..(offset + count) {
            butterfly_twiddle(values, twiddles[i], j, stride);
        }
    }
}

pub fn get_twiddles(root: FieldElement, size: usize) -> Vec<FieldElement> {
    assert!(size.is_power_of_two());
    assert!(FieldElement::exp(root, size as u128) == FieldElement::ONE);
    let mut twiddles = FieldElement::get_power_series(root, size / 2);
    permute(&mut twiddles);
    twiddles
}

pub fn get_inv_twiddles(root: FieldElement, size: usize) -> Vec<FieldElement> {
    let inv_root = FieldElement::exp(root, (size - 1) as u128);
    get_twiddles(inv_root, size)
}

pub fn permute(v: &mut [FieldElement]) {
    let n = v.len();
    for i in 0..n {
        let j = permute_index(n, i);
        if j > i {
            v.swap(i, j);
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn permute_index(size: usize, index: usize) -> usize {
    debug_assert!(index < size);
    if size == 1 {
        return 0;
    }
    debug_assert!(size.is_power_of_two());
    let bits = size.trailing_zeros() as usize;
    index.reverse_bits() >> (USIZE_BITS - bits)
}

#[inline(always)]
fn butterfly(values: &mut [FieldElement], offset: usize, stride: usize) {
    let i = offset;
    let j = offset + stride;
    let temp = values[i];
    values[i] = temp + values[j];
    values[j] = temp - values[j];
}

#[inline(always)]
fn butterfly_twiddle(
    values: &mut [FieldElement],
    twiddle: FieldElement,
    offset: usize,
    stride: usize,
) {
    let i = offset;
    let j = offset + stride;
    let temp = values[i];
    values[j] = values[j] * twiddle;
    values[i] = temp + values[j];
    values[j] = temp - values[j];
}
