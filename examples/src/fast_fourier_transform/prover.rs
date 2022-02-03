// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::{
    get_power_series, rescue, BaseElement, FieldElement, FFTAir, ProofOptions, Prover,
    StarkField, TraceTable, air::PublicInputs,
};

#[cfg(feature = "concurrent")]
use winterfell::iterators::*;
use winterfell::{math::log2, Trace};

// CONSTANTS
// ================================================================================================

const TWO: BaseElement = BaseElement::new(2);
const ZERO_KEY: [BaseElement; 2] = [BaseElement::ZERO, BaseElement::ZERO];


// FFT PROVER
// ================================================================================================

pub struct FFTProver {
    options: ProofOptions,
    degree: usize
}

impl FFTProver {
    pub fn new(
        options: ProofOptions,
        degree: usize,
    ) -> Self {
        Self {
            options,
            degree,
        }
    }

    pub fn build_trace(
        &self,
        omega: BaseElement,
        coefficients: &[BaseElement],
        degree: usize,
    ) -> TraceTable<BaseElement> {
        // allocate memory to hold the trace table
        let trace_width = degree + 3;
        let trace_length = (log2(degree) + 1).try_into().unwrap();
        let mut trace = TraceTable::new(trace_width, trace_length);

        // let powers_of_two = get_power_series(TWO, 128);
        let mut inputs = coefficients;
        inputs[inputs.len()] = omega;
        inputs[inputs.len()] = BaseElement::ONE;
        inputs[inputs.len()] = BaseElement::ZERO;
        // To prove fft(omega, coeff: &[BaseElement], degree) = output: &[BaseElement]
        trace.fill(
            |state| {
                for i in 0..inputs.len() {
                    state[i] = inputs[i]
                }
            },
            |step, state| {
                apply_fft(step, state, degree)
            }
        );
        trace
    }

    fn get_initial_coeffs(&self, trace: &TraceTable<BaseElement>) -> Vec<BaseElement> {
        (0..self.degree).map(|x| trace.get(x, 0)).collect()
    }

    fn get_final_outputs(&self, trace: &TraceTable<BaseElement>) -> Vec<BaseElement> {
        (0..self.degree).map(|x| trace.get(x, trace.length() - 1)).collect()
    }

    fn get_omega(&self, trace: &TraceTable<BaseElement>) -> BaseElement {
        trace.get(self.degree + 1, 0)
    }

}




impl Prover for FFTProver {
    type BaseField = BaseElement;
    type Air = FFTAir;
    type Trace = TraceTable<BaseElement>;

    fn get_pub_inputs(&self, trace: &Self::Trace) -> PublicInputs {
        PublicInputs {
            coefficients: self.get_initial_coeffs(trace),
            omega: self.get_omega(trace),
            degree: self.degree,
            output_evals: self.get_final_outputs(trace),
        }
    }

    fn options(&self) -> &ProofOptions {
        &self.options
    }
}


// TRANSITION FUNCTION
// ================================================================================================
fn apply_fft(step: usize, state: &mut [BaseElement], degree: usize) {
    let omega = state[degree];
    state[degree + 2] = state[degree + 2] + BaseElement::ONE;

    // Swapping for the butterfly network
    if step == 0 {
        let log_degree= log2(degree);
        for i in 0..degree {
            let rev = reverse(i, log_degree);
            if i < rev {
                swap(i, rev, state);
            }
        }
        // Calculate the curr_omega to be used in the next step
        let next_omega = omega.exp((degree/(1 << (step + 1))).try_into().unwrap());
        state[degree + 1] = next_omega;
        return
    }
    
    let curr_omega = state[degree + 1];

    // actual fft
    let jump = 1 << step;
    let counter = 0;
    while counter < degree {
        let mut curr_pow = BaseElement::ONE;
        for j in 0..(1 << (step - 1)) {
            let u = state[counter + j];
            let v = state[counter + j + (1 << (step - 1))] * curr_pow;
            state[counter + j] = u + v;
            state[counter + j + (1 << (step - 1))] = u - v;
            curr_pow = curr_pow * curr_omega;
        }
        counter = counter + jump;
    }

        // Calculate the curr_omega to be used in the next step
        let next_omega = omega.exp((degree/(1 << (step + 1))).try_into().unwrap());
        state[degree + 1] = next_omega;

}

// // HELPER FUNCTIONS
// // ================================================================================================

fn reverse(index: usize, log_degree: u32) -> usize {
    let mut return_index = 0;
    for i in 0..log_degree {
        if index & (1 << i) != 0 {
            return_index = return_index | (1 << (log_degree - 1 - i));
        }
    }
    return_index
}


fn swap(pos1: usize, pos2: usize, state: &mut [BaseElement]) {
    let temp = state[pos1];
    state[pos1] = state[pos2];
    state[pos2] = state[pos1];

}