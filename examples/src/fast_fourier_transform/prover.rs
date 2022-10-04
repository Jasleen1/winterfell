// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use core::num;
use std::convert::TryInto;

use winterfell::{math::{log2, fft}, TraceTable};

use super::{
    BaseElement, FieldElement, ProofOptions,
    Prover, PublicInputs, FFTAir, Trace, apply_fft_permutation, fill_fft_indices, apply_fft_calculation, apply_fft_inv_permutation, apply_bit_rev_copy_permutation,
};



// RESCUE PROVER
// ================================================================================================

pub struct FFTProver {
    options: ProofOptions,
}

impl FFTProver {
    pub fn new(options: ProofOptions) -> Self {
        Self { options }
    }

    pub fn build_trace(
        &self,
        omega: BaseElement,
        fft_inputs: &[BaseElement],
        result: &[BaseElement],
    ) -> TraceTable<BaseElement> {
        // allocate memory to hold the trace table
        let trace_length = fft_inputs.len();
        let log_trace_length: usize = log2(trace_length).try_into().unwrap();
        // For all but the last step, one step to write down the FFT layer and one to write the permutation.
        // The last step is to write down the row numbers.
        let trace_width = 2*log_trace_length + 3;
        let mut trace = TraceTable::new(trace_width, trace_length);
        let last_permutation_step = trace_width - 3;
        let non_fft_step = trace_width - 2;

        trace.fill(
            |state| {
                for i in 0..trace_length {
                    state[i] = fft_inputs[i];
                }
            },
            |step, state| {
                // execute the transition function for all steps
                match step % 2 {
                    // For each even step, we would like to permute the previous col depending on what the step number is.
                    0 => {
                        if step == 0 {
                            // To do iteratative FFT, the first step is to apply this permutation.
                            apply_bit_rev_copy_permutation(state);
                        }
                        if step != 0 {
                            // Undo the permutation from last time, since you put 
                            // together values that would have actually been far apart
                            apply_fft_inv_permutation(state, step);
                        }
                        if step != last_permutation_step {
                            // Lay the values that are computed upon together, 
                            // next to each other
                            apply_fft_permutation(state, step);
                        }
                    },
                    // For each odd step, we would like to do the FFT operation with adjacent values.
                    1 => {
                        if step != non_fft_step {
                            apply_fft_calculation(state, step, omega);
                        }
                        else {
                            fill_fft_indices(state);
                        }
                        
                    },
                    // Required by rust since the type usize is unbounded and we need to be exhaustive with match.
                    _ => {},
                };
            },
        );

        for row in 0..trace_length {
            debug_assert_eq!(trace.get(get_results_col_idx(trace_length), row), result[row]);
        }

        trace
    }

}

impl Prover for FFTProver {
    type BaseField = BaseElement;
    type Air = FFTAir;
    type Trace = TraceTable<BaseElement>;

    fn get_pub_inputs(&self, trace: &Self::Trace) -> PublicInputs {
        let last_fft_state = get_results_col_idx(trace.length());
        let num_inputs = trace.length();
        let mut fft_input_vec = vec![BaseElement::ONE; num_inputs];
        trace.read_row_into(0, &mut fft_input_vec);
        let mut fft_output_vec = vec![BaseElement::ONE; num_inputs];
        trace.read_row_into(last_fft_state, &mut fft_output_vec);
        PublicInputs {
            num_inputs,
            fft_inputs: fft_input_vec,
            result: fft_output_vec,
        }
    }

    fn options(&self) -> &ProofOptions {
        &self.options
    }
}

pub(crate) fn get_results_col_idx(num_fft_inputs: usize) -> usize {
    let log_trace_length: usize = log2(num_fft_inputs).try_into().unwrap();
    2*log_trace_length + 1
}

pub(crate) fn get_num_cols(num_fft_inputs: usize) -> usize {
    let log_trace_length: usize = log2(num_fft_inputs).try_into().unwrap();
    2*log_trace_length + 3
}