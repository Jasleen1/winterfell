// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use core::num;
use std::convert::TryInto;

use winterfell::math::{fft, log2};

use super::{
    apply_bit_rev_copy_permutation, apply_fft_calculation, apply_fft_inv_permutation,
    apply_fft_permutation, fill_fft_indices, BaseElement, FFTRapsAir, FFTTraceTable, FieldElement,
    ProofOptions, Prover, PublicInputs, Trace,
};

// RESCUE PROVER
// ================================================================================================

pub struct FFTRapsProver {
    options: ProofOptions,
}

impl FFTRapsProver {
    pub fn new(options: ProofOptions) -> Self {
        Self { options }
    }

    pub fn build_trace(
        &self,
        omega: BaseElement,
        fft_inputs: &[BaseElement],
        result: &[BaseElement],
    ) -> FFTTraceTable<BaseElement> {
        // allocate memory to hold the trace table
        let trace_length = fft_inputs.len();
        let log_trace_length: usize = log2(trace_length).try_into().unwrap();
        // For all but the last step, one step to write down the FFT layer and one to write the permutation.
        // The last step is to write down the row numbers.
        let trace_width = 3 * log_trace_length + 2;
        let mut trace = FFTTraceTable::new(trace_width, trace_length);
        let last_forward_permutation_step = trace_width - 5;
        let last_back_permutation_step = trace_width - 3;
        let non_fft_step = trace_width - 2;

        trace.fill_cols(
            |state| {
                for i in 0..trace_length {
                    state[i] = fft_inputs[i];
                }
            },
            |step, state| {
                // execute the transition function for all steps
                match step % 3 {
                    // For each even step, we would like to permute the previous col depending on what the step number is.
                    0 => {
                        if step == 0 || step == non_fft_step {
                            // To do iteratative FFT, the first step is to apply this permutation.
                            // At the last step, we also want to apply this permutation to a column that
                            // keeps track of indices. This is useful when doing permutation checks.
                            apply_bit_rev_copy_permutation(state);
                        }
                        if step >= 3 && step != non_fft_step {
                            apply_fft_calculation(state, step, omega);
                        }
                    }
                    // For each 1 (mod 3) step, except the first,
                    // we would like to do the an inverse fft permutation.
                    // For the first step, we would like to do the FFT operation with adjacent values.
                    1 => {
                        if step == 1 {
                            apply_fft_calculation(state, step, omega);
                        } else {
                            assert!(
                                step <= last_back_permutation_step,
                                "Looks like you're doing an inverse permutation somewhere 
                                higher than the permitted {}. You tried {}",
                                last_back_permutation_step,
                                step
                            );
                            apply_fft_inv_permutation(state, step);
                        }
                    }
                    2 => {
                        if step <= last_forward_permutation_step {
                            apply_fft_permutation(state, step);
                        } else {
                            fill_fft_indices(state);
                        }
                    }
                    // Required by rust since the type usize is unbounded and
                    // we need to be exhaustive with match.
                    _ => {}
                };
            },
        );
        let mut last_trace_col = vec![BaseElement::ONE; trace_length];
        trace.read_col_into(get_results_col_idx(trace_length), &mut last_trace_col);
        for row in 0..trace_length {
            debug_assert_eq!(
                trace.get(get_results_col_idx(trace_length), row),
                result[row]
            );
        }

        trace
    }
}

impl Prover for FFTRapsProver {
    type BaseField = BaseElement;
    type Air = FFTRapsAir;
    type Trace = FFTTraceTable<BaseElement>;

    fn get_pub_inputs(&self, trace: &Self::Trace) -> PublicInputs {
        let last_fft_state = get_results_col_idx(trace.length());
        let num_inputs = trace.length();
        let mut fft_input_vec = vec![BaseElement::ONE; num_inputs];
        trace.read_col_into(0, &mut fft_input_vec);
        let mut fft_output_vec = vec![BaseElement::ONE; num_inputs];
        trace.read_col_into(last_fft_state, &mut fft_output_vec);
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
    3 * log_trace_length - 1
}

pub(crate) fn get_num_cols(num_fft_inputs: usize) -> usize {
    let log_trace_length: usize = log2(num_fft_inputs).try_into().unwrap();
    3 * log_trace_length + 2
}

pub(crate) fn get_num_steps(num_fft_inputs: usize) -> usize {
    let log_trace_length: usize = log2(num_fft_inputs).try_into().unwrap();
    log_trace_length
}
