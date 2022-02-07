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

// FIXME

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
        data_size: usize,
    ) -> TraceTable<BaseElement> {
        // allocate memory to hold the trace table
        let trace_length = (log2(data_size) + 1).try_into().unwrap();
        // Degree to store coeffs + 1 col to store omega + 
        // 1 col to store omega^{1 << step_number} + 1 col for step number
        // The foll tuple for each step number (pos - step_number, f(pos - step number), selector_bit)
        // where pos iterates over the set of step numbers in each row,
        // f(x) = { 1 if x == 0, x^-1 if x =/= 0
        // So 1 - f(x) * x = 1 only when x = 0 and 0 otherwise. 
        let trace_width = data_size + 3 + 3 * trace_length;
        let mut trace = TraceTable::new(trace_width, trace_length);
        // Layout 
        // | -- 0 to (data_size - 1) is coefficients 
        // | -- position # data_size contains omega
        // | -- position # data_size + 1 contains the layer omega
        // FIXME finish
        // FIXME Move to separate function
        // data_size is N in the description of an FFT
        let mut inputs = vec![BaseElement::ZERO; trace_width];
        inputs[..data_size].copy_from_slice(coefficients);
        let omega_pos = data_size;
        inputs[omega_pos] = omega;

        // Omega^{data_size/(1<<step_number)}
        let layer_omega = data_size + 1;
        inputs[layer_omega] = BaseElement::ONE;

        // This is just the step counter aka AIR layer number within the trace
        let air_step_counter_pos = data_size + 2;
        inputs[air_step_counter_pos] = BaseElement::ZERO;
        let trace_128: u128 = trace_length.try_into().unwrap();

        // Now we've filled data_size + 3 positions 
        
        fill_selector_info(&mut inputs, BaseElement::ZERO, trace_128, data_size);
        // To prove fft(omega, coeff: &[BaseElement], degree) = output: &[BaseElement]
        trace.fill(
            |state| {
                for i in 0..inputs.len() {
                    state[i] = inputs[i]
                }
            },
            |step, state| {
                apply_fft(step, state, data_size, trace_128)
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
            // output_evals: self.get_final_outputs(trace),
        }
    }

    fn options(&self) -> &ProofOptions {
        &self.options
    }
}


// TRANSITION FUNCTION
// ================================================================================================
fn apply_fft(step: usize, state: &mut [BaseElement], degree: usize, trace_length: u128) {
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
    let mut counter = 0;
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
    let step_u128: u128 = step.try_into().unwrap();
    let step_base_elt = BaseElement::from(step_u128);
    fill_selector_info(state, step_base_elt, trace_length, degree);
    
    // Calculate the curr_omega to be used in the next step
    let next_omega = omega.exp((degree/(1 << (step + 1))).try_into().unwrap());
    state[degree + 1] = next_omega;

}

// // HELPER FUNCTIONS
// // ================================================================================================

fn fill_selector_info(state: &mut [BaseElement], step: BaseElement, trace_128: u128, degree: usize) {
    for i in 0..trace_128 {
        let i_group_elt = BaseElement::from(i) - step;
        let i_inv_try = {
            if i_group_elt == BaseElement::ZERO {
                BaseElement::ONE
            }
            else {
                i_group_elt.inv()
            }
        };
        let selector_bit = BaseElement::ONE - (i_group_elt * i_inv_try);
        let i_usize: usize = i.try_into().unwrap();
        state[get_count_diff_pos(i_usize, degree)] = i_group_elt;
        state[get_inv_pos(i_usize, degree)] = i_inv_try;
        state[get_selector_pos(i_usize, degree)] = selector_bit;

    }
}

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
    state[pos2] = temp;

}

fn get_count_diff_pos(i: usize, degree: usize) -> usize {
    degree + 3 + 3*i
}

fn get_inv_pos(i: usize, degree: usize) -> usize {
    degree + 3 + 3*i + 1
}

fn get_selector_pos(i: usize, degree: usize) -> usize {
    degree + 3 + 3*i + 2
}

