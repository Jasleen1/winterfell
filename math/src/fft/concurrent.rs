use crate::field::{FieldElement, StarkField};
use rayon::prelude::*;

// CONCURRENT VERSIONS OF FFT FUNCTIONS
// ================================================================================================

pub fn evaluate_poly<B: StarkField, E: FieldElement + From<B>>(p: &mut [E], twiddles: &[B]) {
    split_radix_fft(p, twiddles);
    permute_values(p);
}

pub fn interpolate_poly<B: StarkField, E: FieldElement + From<B>>(v: &mut [E], inv_twiddles: &[B]) {
    split_radix_fft(v, inv_twiddles);
    let inv_length = E::inv((v.len() as u64).into());
    v.par_iter_mut().for_each(|e| {
        *e = *e * inv_length;
    });
    permute_values(v);
}

pub fn permute_values<E: FieldElement>(v: &mut [E]) {
    let n = v.len();
    let num_batches = rayon::current_num_threads().next_power_of_two();
    let batch_size = n / num_batches;
    rayon::scope(|s| {
        for batch_idx in 0..num_batches {
            // create another mutable reference to the slice of values to use in a new thread; this
            // is OK because we never write the same positions in the slice from different threads
            let values = unsafe { &mut *(&mut v[..] as *mut [E]) };
            s.spawn(move |_| {
                let batch_start = batch_idx * batch_size;
                let batch_end = batch_start + batch_size;
                for i in batch_start..batch_end {
                    let j = super::permute_index(n, i);
                    if j > i {
                        values.swap(i, j);
                    }
                }
            });
        }
    });
}

// SPLIT-RADIX FFT
// ================================================================================================

/// In-place recursive FFT with permuted output.
/// Adapted from: https://github.com/0xProject/OpenZKP/tree/master/algebra/primefield/src/fft
pub fn split_radix_fft<B: StarkField, E: FieldElement + From<B>>(values: &mut [E], twiddles: &[B]) {
    // generator of the domain should be in the middle of twiddles
    let n = values.len();
    let g = E::from(twiddles[twiddles.len() / 2]);
    debug_assert_eq!(g.exp((n as u32).into()), E::ONE);

    let inner_len = 1_usize << (n.trailing_zeros() / 2);
    let outer_len = n / inner_len;
    let stretch = outer_len / inner_len;
    debug_assert!(outer_len == inner_len || outer_len == 2 * inner_len);
    debug_assert_eq!(outer_len * inner_len, n);

    // transpose inner x inner x stretch square matrix
    transpose_square_stretch(values, inner_len, stretch);

    // apply inner FFTs
    values
        .par_chunks_mut(outer_len)
        .for_each(|row| super::fft_in_place(row, &twiddles, stretch, stretch, 0));

    // transpose inner x inner x stretch square matrix
    transpose_square_stretch(values, inner_len, stretch);

    // apply outer FFTs
    values
        .par_chunks_mut(outer_len)
        .enumerate()
        .for_each(|(i, row)| {
            if i > 0 {
                let i = super::permute_index(inner_len, i);
                let inner_twiddle = g.exp((i as u32).into());
                let mut outer_twiddle = inner_twiddle;
                for element in row.iter_mut().skip(1) {
                    *element = *element * outer_twiddle;
                    outer_twiddle = outer_twiddle * inner_twiddle;
                }
            }
            super::fft_in_place(row, &twiddles, 1, 1, 0)
        });
}

// TRANSPOSING
// ================================================================================================

fn transpose_square_stretch<T>(matrix: &mut [T], size: usize, stretch: usize) {
    assert_eq!(matrix.len(), size * size * stretch);
    match stretch {
        1 => transpose_square_1(matrix, size),
        2 => transpose_square_2(matrix, size),
        _ => unimplemented!("only stretch sizes 1 and 2 are supported"),
    }
}

fn transpose_square_1<T>(matrix: &mut [T], size: usize) {
    debug_assert_eq!(matrix.len(), size * size);
    if size % 2 != 0 {
        unimplemented!("odd sizes are not supported");
    }

    // iterate over upper-left triangle, working in 2x2 blocks
    for row in (0..size).step_by(2) {
        let i = row * size + row;
        matrix.swap(i + 1, i + size);
        for col in (row..size).step_by(2).skip(1) {
            let i = row * size + col;
            let j = col * size + row;
            matrix.swap(i, j);
            matrix.swap(i + 1, j + size);
            matrix.swap(i + size, j + 1);
            matrix.swap(i + size + 1, j + size + 1);
        }
    }
}

fn transpose_square_2<T>(matrix: &mut [T], size: usize) {
    debug_assert_eq!(matrix.len(), 2 * size * size);

    // iterate over upper-left triangle, working in 1x2 blocks
    for row in 0..size {
        for col in (row..size).skip(1) {
            let i = (row * size + col) * 2;
            let j = (col * size + row) * 2;
            matrix.swap(i, j);
            matrix.swap(i + 1, j + 1);
        }
    }
}
