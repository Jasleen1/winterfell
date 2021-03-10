use crate::field::FieldElement;

#[cfg(feature = "concurrent")]
mod concurrent;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================
const USIZE_BITS: usize = 0_usize.count_zeros() as usize;
const MAX_LOOP: usize = 256;
pub const MIN_CONCURRENT_SIZE: usize = 1024;

// POLYNOMIAL EVALUATION AND INTERPOLATION
// ================================================================================================

/// Evaluates polynomial `p` using FFT algorithm; the evaluation is done in-place, meaning
/// `p` is updated with results of the evaluation.
///
/// When `concurrent` feature is enabled, the evaluation uses as many threads as are
/// available in Rayon's global thread pool (usually as many threads as logical cores).
/// Otherwise, the evaluation is done in a single thread
pub fn evaluate_poly<E: FieldElement>(p: &mut [E], twiddles: &[E]) {
    debug_assert!(
        p.len().is_power_of_two(),
        "number of coefficients must be a power of 2"
    );
    debug_assert_eq!(p.len(), twiddles.len() * 2, "invalid number of twiddles");

    // when `concurrent` feature is enabled, run the concurrent version of evaluate_poly; unless
    // the polynomial is small, then don't bother with the concurrent version
    if cfg!(feature = "concurrent") && p.len() >= MIN_CONCURRENT_SIZE {
        #[cfg(feature = "concurrent")]
        concurrent::evaluate_poly(p, twiddles);
    } else {
        fft_in_place(p, twiddles, 1, 1, 0);
        permute_values(p);
    }
}

/// Uses FFT algorithm to interpolate a polynomial from provided values `v`; the interpolation
/// is done in-place, meaning `v` is updated with polynomial coefficients.
///
/// When `concurrent` feature is enabled, the interpolation uses as many threads as are
/// available in Rayon's global thread pool (usually as many threads as logical cores).
/// Otherwise, the interpolation is done in a single thread
pub fn interpolate_poly<E: FieldElement>(v: &mut [E], inv_twiddles: &[E]) {
    debug_assert!(
        v.len().is_power_of_two(),
        "number of values must be a power of 2"
    );
    debug_assert_eq!(
        v.len(),
        inv_twiddles.len() * 2,
        "invalid number of twiddles"
    );

    // when `concurrent` feature is enabled, run the concurrent version of interpolate_poly;
    // unless the number of evaluations is small, then don't bother with the concurrent version
    if cfg!(feature = "concurrent") && v.len() >= MIN_CONCURRENT_SIZE {
        #[cfg(feature = "concurrent")]
        concurrent::interpolate_poly(v, inv_twiddles);
    } else {
        fft_in_place(v, &inv_twiddles, 1, 1, 0);
        let inv_length = E::inv((v.len() as u64).into());
        for e in v.iter_mut() {
            *e = *e * inv_length;
        }
        permute_values(v);
    }
}

// TWIDDLES
// ================================================================================================

pub fn get_twiddles<E: FieldElement>(root: E, size: usize) -> Vec<E> {
    debug_assert!(size.is_power_of_two(), "domain size must be a power of 2");
    debug_assert_eq!(
        E::ONE,
        E::exp(root, (size as u32).into()),
        "invalid root of unity"
    );
    let mut twiddles;
    if cfg!(feature = "concurrent") && size >= MIN_CONCURRENT_SIZE {
        // this makes compiler happy, but otherwise is pointless
        #[cfg(not(feature = "concurrent"))]
        {
            twiddles = Vec::new();
        }
        #[cfg(feature = "concurrent")]
        {
            twiddles = concurrent::get_twiddles(root, size);
        }
    } else {
        twiddles = E::get_power_series(root, size / 2);
        permute_values(&mut twiddles);
    }
    twiddles
}

pub fn get_inv_twiddles<E: FieldElement>(root: E, size: usize) -> Vec<E> {
    let inv_root = E::exp(root, (size as u32 - 1).into());
    get_twiddles(inv_root, size)
}

pub fn permute<E: FieldElement>(v: &mut [E]) {
    if cfg!(feature = "concurrent") && v.len() >= MIN_CONCURRENT_SIZE {
        #[cfg(feature = "concurrent")]
        concurrent::permute_values(v);
    } else {
        permute_values(v);
    }
}

// CORE FFT ALGORITHM
// ================================================================================================

/// In-place recursive FFT with permuted output.
/// Adapted from: https://github.com/0xProject/OpenZKP/tree/master/algebra/primefield/src/fft
fn fft_in_place<E: FieldElement>(
    values: &mut [E],
    twiddles: &[E],
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

// HELPER FUNCTIONS
// ================================================================================================

fn permute_values<T>(values: &mut [T]) {
    let n = values.len();
    for i in 0..n {
        let j = permute_index(n, i);
        if j > i {
            values.swap(i, j);
        }
    }
}

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
fn butterfly<E: FieldElement>(values: &mut [E], offset: usize, stride: usize) {
    let i = offset;
    let j = offset + stride;
    let temp = values[i];
    values[i] = temp + values[j];
    values[j] = temp - values[j];
}

#[inline(always)]
fn butterfly_twiddle<E: FieldElement>(values: &mut [E], twiddle: E, offset: usize, stride: usize) {
    let i = offset;
    let j = offset + stride;
    let temp = values[i];
    values[j] = values[j] * twiddle;
    values[i] = temp + values[j];
    values[j] = temp - values[j];
}
