// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::convert::TryInto;

use winterfell::math::{log2, fft};

use super::{
    BaseElement, FieldElement, ProofOptions,
    Prover, PublicInputs, FFTTraceTable, FFTRapsAir, Trace, apply_fft_permutation, fill_fft_indices, apply_fft_calculation,
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
    // TODO #2 is to implement this function
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
        let trace_width = 2*log_trace_length;
        let mut trace = FFTTraceTable::new(trace_width, trace_length);
        let last_step = trace_width - 2;

        trace.fill_cols(
            |state| {
                for i in 0..trace_length {
                    state[i] = fft_inputs[i];
                }
            },
            |step, state| {
                // execute the transition function for all steps
                // 
                match step % 2 {
                    // For each even step, we would like to permute the previous col depending on what the step number is.
                    0 => {
                        if step != last_step {
                            apply_fft_permutation(state, step);
                        }
                        else {
                            fill_fft_indices(state);
                        }
                    },
                    // For each odd step, we would like to do the FFT operation with adjacent values.
                    1 => {
                        apply_fft_calculation(state, step, omega);
                    },
                    // Required by rust since the type usize is unbounded and we need to be exhaustive with match.
                    _ => {},
                };
            },
        );

        // debug_assert_eq!(trace.get(0, trace_length - 1), result[0][0]);
        // debug_assert_eq!(trace.get(1, trace_length - 1), result[0][1]);

        // debug_assert_eq!(trace.get(4, trace_length - 1), result[1][0]);
        // debug_assert_eq!(trace.get(5, trace_length - 1), result[1][1]);

        trace
    }
}

impl Prover for FFTRapsProver {
    type BaseField = BaseElement;
    type Air = FFTRapsAir;
    type Trace = FFTTraceTable<BaseElement>;

    fn get_pub_inputs(&self, trace: &Self::Trace) -> PublicInputs {
        let last_step = trace.length() - 1;
        // PublicInputs {
        //     result: [
        //         [trace.get(0, last_step), trace.get(1, last_step)],
        //         [trace.get(4, last_step), trace.get(5, last_step)],
        //     ],
        // }
        unimplemented!()
    }

    fn options(&self) -> &ProofOptions {
        &self.options
    }
}


fn handle_even_steps(step: usize, state: &mut [BaseElement]) {
    // If this is not the last step, then permute

    // If this is the last step, just fill it in with field representations of indices
    unimplemented!()
}
