// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use core_utils::{collections::Vec, uninit_vector};
use winterfell::{
    math::{fields::f128::BaseElement, log2, FieldElement, StarkField},
    EvaluationFrame, Matrix, Trace, TraceInfo, TraceLayout,
};

use crate::{
    fast_fourier_transform_raps::prover::get_num_steps, utils::fast_fourier_transform::bit_reverse,
};

use super::{
    air::FFTRapsAir,
    get_fft_inv_permutation_locs, get_fft_permutation_locs,
    prover::{get_num_cols, FFTRapsProver},
};

// RAP TRACE TABLE
// ================================================================================================
/// A concrete implementation of the [Trace] trait supporting custom RAPs.
///
/// This implementation should be sufficient for most use cases.
/// To create a trace table, you can use [RapTraceTable::new()] function, which takes trace
/// width and length as parameters. This function will allocate memory for the trace, but will not
/// fill it with data. To fill the execution trace, you can use the [fill()](RapTraceTable::fill)
/// method, which takes two closures as parameters:
///
/// 1. The first closure is responsible for initializing the first state of the computation
///    (the first row of the execution trace).
/// 2. The second closure receives the previous state of the execution trace as input, and must
///    update it to the next state of the computation.
///
/// You can also use [RapTraceTable::with_meta()] function to create a blank execution trace.
/// This function work just like [RapTraceTable::new()] function, but also takes a metadata
/// parameter which can be an arbitrary sequence of bytes up to 64KB in size.
pub struct FFTTraceTable<B: StarkField> {
    layout: TraceLayout,
    trace: Matrix<B>,
    meta: Vec<u8>,
}

impl<B: StarkField> FFTTraceTable<B> {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new execution trace of the specified width and length.
    ///
    /// This allocates all the required memory for the trace, but does not initialize it. It is
    /// expected that the trace will be filled using one of the data mutator methods.
    ///
    /// # Panics
    /// Panics if:
    /// * `width` is zero or greater than 255.
    /// * `length` is smaller than 8, greater than biggest multiplicative subgroup in the field
    ///   `B`, or is not a power of two.
    pub fn new(width: usize, length: usize) -> Self {
        Self::with_meta(width, length, vec![])
    }

    /// Creates a new execution trace of the specified width and length, and with the specified
    /// metadata.
    ///
    /// This allocates all the required memory for the trace, but does not initialize it. It is
    /// expected that the trace will be filled using one of the data mutator methods.
    ///
    /// # Panics
    /// Panics if:
    /// * `width` is zero or greater than 255.
    /// * `length` is smaller than 8, greater than the biggest multiplicative subgroup in the
    ///   field `B`, or is not a power of two.
    /// * Length of `meta` is greater than 65535;
    pub fn with_meta(width: usize, length: usize, meta: Vec<u8>) -> Self {
        assert!(
            width > 0,
            "execution trace must consist of at least one column"
        );
        assert!(
            width <= TraceInfo::MAX_TRACE_WIDTH,
            "execution trace width cannot be greater than {}, but was {}",
            TraceInfo::MAX_TRACE_WIDTH,
            width
        );
        assert!(
            length >= TraceInfo::MIN_TRACE_LENGTH,
            "execution trace must be at least {} steps long, but was {}",
            TraceInfo::MIN_TRACE_LENGTH,
            length
        );
        assert!(
            length.is_power_of_two(),
            "execution trace length must be a power of 2"
        );
        assert!(
            log2(length) as u32 <= B::TWO_ADICITY,
            "execution trace length cannot exceed 2^{} steps, but was 2^{}",
            B::TWO_ADICITY,
            log2(length)
        );
        assert!(
            meta.len() <= TraceInfo::MAX_META_LENGTH,
            "number of metadata bytes cannot be greater than {}, but was {}",
            TraceInfo::MAX_META_LENGTH,
            meta.len()
        );

        let columns = unsafe { (0..width).map(|_| uninit_vector(length)).collect() };
        let num_aux_vals = Self::get_aux_col_width_fft(width);
        let num_aux_rands = Self::get_num_rand_fft(width);
        Self {
            layout: TraceLayout::new(width, [num_aux_vals], [num_aux_rands]),
            trace: Matrix::new(columns),
            meta,
        }
    }

    // We want to show that the column for each fft step was permuted correctly
    // Keeping a column with step numbers for now.
    fn get_aux_col_width_fft(width: usize) -> usize {
        // width - 2
        // 3 + 3
        //3 + 3*((width - 4)/2+1)
        3 + 2 * 3 * ((width - 2) / 3 - 1)
    }

    // We want to show that the column for each fft step was permuted correctly,
    // so we'll want terms like alpha_0 * loc(col, step) + alpha_1 * val(col, step) + gamma
    fn get_num_rand_fft(width: usize) -> usize {
        // 3 * (width - 2)
        // 3 + 3*((width - 4)/2+1)
        3
    }

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Fill all rows in the execution trace.
    ///
    /// The rows are filled by executing the provided closures as follows:
    /// - `init` closure is used to initialize the first col of the trace; it receives a mutable
    ///   reference to the first state initialized to all zeros. The contents of the state are
    ///   copied into the first row of the trace after the closure returns.
    /// - `update` closure is used to populate all subsequent rows of the trace; it receives two
    ///   parameters:
    ///   - index of the last updated row (starting with 0).
    ///   - a mutable reference to the last updated state; the contents of the state are copied
    ///     into the next row of the trace after the closure returns.
    pub fn fill_cols<I, U>(&mut self, init: I, update: U)
    where
        I: Fn(&mut [B]),
        U: Fn(usize, &mut [B]),
    {
        let mut state = vec![B::ZERO; self.length()];
        init(&mut state);
        self.update_col(0, &state);

        for i in 0..self.width() - 1 {
            update(i, &mut state);
            self.update_col(i + 1, &state);
        }
    }

    /// Updates a single col in the execution trace with provided data.
    pub fn update_col(&mut self, step: usize, state: &[B]) {
        update_matrix_col(&mut self.trace, step, state);
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the number of columns in this execution trace.
    pub fn width(&self) -> usize {
        self.main_trace_width()
    }

    /// Returns value of the cell in the specified column at the specified row of this trace.
    pub fn get(&self, column: usize, step: usize) -> B {
        self.trace.get(column, step)
    }

    /// Reads a single row from this execution trace into the provided target.
    pub fn read_row_into(&self, step: usize, target: &mut [B]) {
        self.trace.read_row_into(step, target);
    }

    /// Reads a single col from this execution trace into the provided target.
    pub fn read_col_into(&self, col_idx: usize, target: &mut [B]) {
        for row_idx in 0..self.trace.num_rows() {
            target[row_idx] = self.trace.get(col_idx, row_idx);
        }
    }
}

// TRACE TRAIT IMPLEMENTATION
// ================================================================================================

impl<B: StarkField> Trace for FFTTraceTable<B> {
    type BaseField = B;

    fn layout(&self) -> &TraceLayout {
        &self.layout
    }

    fn length(&self) -> usize {
        self.trace.num_rows()
    }

    fn meta(&self) -> &[u8] {
        &self.meta
    }

    fn read_main_frame(&self, row_idx: usize, frame: &mut EvaluationFrame<Self::BaseField>) {
        let next_row_idx = (row_idx + 1) % self.length();
        self.trace.read_row_into(row_idx, frame.current_mut());
        self.trace.read_row_into(next_row_idx, frame.next_mut());
    }

    fn main_segment(&self) -> &Matrix<B> {
        &self.trace
    }

    fn build_aux_segment<E>(
        &mut self,
        aux_segments: &[Matrix<E>],
        rand_elements: &[E],
    ) -> Option<Matrix<E>>
    where
        E: FieldElement<BaseField = Self::BaseField>,
    {
        // We only have one auxiliary segment for this example
        if !aux_segments.is_empty() {
            return None;
        }
        let fft_width = get_num_cols(self.length());
        let num_steps = get_num_steps(self.length());
        let mut current_row = unsafe { uninit_vector(self.width()) };
        // let mut next_row = unsafe { uninit_vector(self.width()) };
        self.read_row_into(0, &mut current_row);
        let mut aux_columns = vec![vec![E::ZERO; self.length()]; self.aux_trace_width()];

        aux_columns[2][0] = E::ONE;
        // Columns storing the copied values for the permutation argument are not necessary, but
        // help understanding the construction of RAPs and are kept for illustrative purposes.
        aux_columns[0][0] = rand_elements[0] * current_row[0].into()
            + rand_elements[1] * current_row[fft_width - 1].into();
        aux_columns[1][0] = rand_elements[0] * current_row[1].into()
            + rand_elements[1] * current_row[fft_width - 2].into();

        // This array contains the backward permutation for jump = 2, ..., log(fft_size)
        let backward_permutation_locs: Vec<Vec<E>> = get_fft_perm_rev_locs(self.length());
        // This array contains the forward permutation for jump = 2, ..., log(fft_size)
        let forward_permutation_locs: Vec<Vec<E>> = get_fft_perm_forward_locs(self.length());

        // These columns will check whether the forward permutation was done correctly
        for step in 2..num_steps + 1 {
            aux_columns[3 * (step - 1)][0] = rand_elements[0]
                * current_row[3 * (step - 1) - 1].into()
                + rand_elements[1] * current_row[fft_width - 2].into();
            aux_columns[3 * (step - 1) + 1][0] = rand_elements[0]
                * current_row[3 * (step - 1)].into()
                + rand_elements[1] * forward_permutation_locs[step - 2][0].into();
            aux_columns[3 * (step - 1) + 2][0] = E::ONE;
        }

        // These columns will be used to check if the backward permutation was done correctly
        for step in 2..num_steps + 1 {
            aux_columns[3 * num_steps + 3 * (step - 2)][0] = rand_elements[0]
                * current_row[3 * (step - 1) + 1].into()
                + rand_elements[1] * backward_permutation_locs[step - 2][0].into();
            aux_columns[3 * num_steps + 3 * (step - 2) + 1][0] = rand_elements[0]
                * current_row[3 * (step - 1) + 2].into()
                + rand_elements[1] * current_row[fft_width - 2].into();
            aux_columns[3 * num_steps + 3 * (step - 2) + 2][0] = E::ONE;
        }

        for index in 1..self.length() {
            self.read_row_into(index, &mut current_row);

            // Checking that column 1 is a bit reverse permutation of column 0.
            aux_columns[0][index] = rand_elements[0] * current_row[0].into()
                + rand_elements[1] * current_row[fft_width - 2].into();
            aux_columns[1][index] = rand_elements[0] * current_row[1].into()
                + rand_elements[1] * current_row[fft_width - 1].into();

            let num = aux_columns[0][index - 1] + rand_elements[2];
            let denom = aux_columns[1][index - 1] + rand_elements[2];

            aux_columns[2][index] = aux_columns[2][index - 1] * num * denom.inv();

            // Fill columns for forward permutations
            for step in 2..num_steps + 1 {
                aux_columns[3 * (step - 1)][index] = rand_elements[0]
                    * current_row[3 * (step - 1) - 1].into()
                    + rand_elements[1] * forward_permutation_locs[step - 2][index].into();
                aux_columns[3 * (step - 1) + 1][index] = rand_elements[0]
                    * current_row[3 * (step - 1)].into()
                    + rand_elements[1] * current_row[fft_width - 2].into();

                let num = aux_columns[3 * (step - 1)][index - 1] + rand_elements[2];
                let denom = aux_columns[3 * (step - 1) + 1][index - 1] + rand_elements[2];

                aux_columns[3 * (step - 1) + 2][index] =
                    aux_columns[3 * (step - 1) + 2][index - 1] * num * denom.inv();
            }

            // Fill columns for backward permutations
            for step in 2..num_steps + 1 {
                aux_columns[3 * num_steps + 3 * (step - 2)][index] = rand_elements[0]
                    * current_row[3 * (step - 1) + 1].into()
                    + rand_elements[1] * backward_permutation_locs[step - 2][index].into();
                aux_columns[3 * num_steps + 3 * (step - 2) + 1][index] = rand_elements[0]
                    * current_row[3 * (step - 1) + 2].into()
                    + rand_elements[1] * current_row[fft_width - 2].into();

                let num = aux_columns[3 * num_steps + 3 * (step - 2)][index - 1] + rand_elements[2];
                let denom =
                    aux_columns[3 * num_steps + 3 * (step - 2) + 1][index - 1] + rand_elements[2];

                aux_columns[3 * num_steps + 3 * (step - 2) + 2][index] =
                    aux_columns[3 * num_steps + 3 * (step - 2) + 2][index - 1] * num * denom.inv();
            }
        }

        Some(Matrix::new(aux_columns))
    }
}

fn update_matrix_col<E: FieldElement>(matrix: &mut Matrix<E>, col_idx: usize, col: &[E]) {
    for row_idx in 0..matrix.num_rows() {
        matrix.set(col_idx, row_idx, col[row_idx]);
    }
}

pub(crate) fn get_fft_perm_rev_locs<E: FieldElement>(fft_size: usize) -> Vec<Vec<E>> {
    let num_fft_steps: usize = log2(fft_size).try_into().unwrap();
    let mut output: Vec<Vec<E>> = vec![];
    for step in 2..num_fft_steps + 1 {
        let mut backward_perm_usize = get_fft_inv_permutation_locs(fft_size, step);

        let backward_perm = backward_perm_usize
            .iter_mut()
            .map(|x| {
                let x_u64: u64 = (*x).try_into().unwrap();
                E::from(x_u64)
            })
            .collect::<Vec<E>>();

        output.push(backward_perm);
    }
    output
}

pub(crate) fn get_fft_perm_forward_locs<E: FieldElement>(fft_size: usize) -> Vec<Vec<E>> {
    let num_fft_steps: usize = log2(fft_size).try_into().unwrap();
    let mut output: Vec<Vec<E>> = vec![];
    for step in 2..num_fft_steps + 1 {
        let mut forward_perm_usize = get_fft_permutation_locs(fft_size, step);

        let forward_perm = forward_perm_usize
            .iter_mut()
            .map(|x| {
                let x_u64: u64 = (*x).try_into().unwrap();
                E::from(x_u64)
            })
            .collect::<Vec<E>>();

        output.push(forward_perm);
    }
    output
}
