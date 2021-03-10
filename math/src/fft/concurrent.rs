use crate::field::FieldElement;
use rayon::prelude::*;

const MIN_CONCURRENT_SIZE: usize = 1024;

pub fn split_radix_fft<F: FieldElement>(values: &mut [F], twiddles: &[F]) {
    let n = values.len();
    debug_assert!(n.is_power_of_two(), "number of values must be a power of 2");
    debug_assert_eq!(n / 2, twiddles.len(), "inconsistent number of twiddles");

    // if we are evaluating a small number of values, run single-threaded version
    if n < MIN_CONCURRENT_SIZE {
        super::fft_in_place(values, twiddles, 1, 1, 0);
    }

    // generator of the domain should be in the middle of twiddles
    let g = twiddles[twiddles.len() / 2];
    debug_assert_eq!(g.exp((n as u32).into()), F::ONE);

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
                let mut outer_twiddle = inner_twiddle.clone();
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

pub fn transpose_square_stretch<T>(matrix: &mut [T], size: usize, stretch: usize) {
    assert_eq!(matrix.len(), size * size * stretch);
    match stretch {
        1 => transpose_square_1(matrix, size),
        2 => transpose_square_2(matrix, size),
        _ => unimplemented!("Only stretch sizes 1 and 2 are supported"),
    }
}

// TODO: Handle odd sizes
fn transpose_square_1<T>(matrix: &mut [T], size: usize) {
    debug_assert_eq!(matrix.len(), size * size);
    if size % 2 != 0 {
        unimplemented!("Odd sizes are not supported");
    }

    // Iterate over upper-left triangle, working in 2x2 blocks
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

    // Iterate over upper-left triangle, working in 1x2 blocks
    for row in 0..size {
        for col in (row..size).skip(1) {
            let i = (row * size + col) * 2;
            let j = (col * size + row) * 2;
            matrix.swap(i, j);
            matrix.swap(i + 1, j + 1);
        }
    }
}
