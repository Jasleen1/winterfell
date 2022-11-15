// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::utils::fast_fourier_transform::bit_reverse;

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
    num_main_trace_rows: usize, 
}

impl FFTProver {
    pub fn new(
        options: ProofOptions,
        num_fft_inputs: usize,
    ) -> Self {
        let num_main_trace_rows = get_num_main_trace_rows(num_fft_inputs);
        Self {
            options,
            num_main_trace_rows,
        }
    }

    pub fn build_trace(
        &self,
        omega: BaseElement,
        fft_inputs: &[BaseElement],
        result: &[BaseElement],
    ) -> TraceTable<BaseElement> {
        let num_fft_inputs = fft_inputs.len();
        // allocate memory to hold the trace table
        let main_trace_length = get_num_main_trace_rows(num_fft_inputs);
        let full_trace_length = get_num_trace_rows(num_fft_inputs);
        // Degree to store coeffs + 1 col to store omega + 
        // 1 col to store omega^{1 << step_number} + 1 col for step number
        // The full tuple for each step number (pos - step_number, f(pos - step number), selector_bit)
        // where pos iterates over the set of step numbers in each row,
        // f(x) = { 1 if x == 0, x^-1 if x =/= 0
        // So 1 - f(x) * x = 1 only when x = 0 and 0 otherwise. 
        let trace_width = get_num_cols(num_fft_inputs);
        // let mut trace = TraceTable::new(trace_width, full_trace_length);
        let mut trace = TraceTable::new(trace_width, full_trace_length);
        // Layout 
        // | -- 0 to (data_size - 1) is coefficients 
        // | -- position # data_size contains omega
        // | -- position # data_size + 1 contains the layer omega
        // FIXME finish
        // FIXME Move to separate function
        // data_size is N in the description of an FFT
        let mut inputs = vec![BaseElement::ZERO; trace_width];
        inputs[..num_fft_inputs].copy_from_slice(fft_inputs);
        // Omega^{data_size/(1<<step_number)}
        let curr_omega_pos = num_fft_inputs;
        inputs[curr_omega_pos] = BaseElement::ONE;

    
        /* 
        // This is just the step counter aka AIR layer number within the trace
        let air_step_counter_pos = data_size + 2;
        inputs[air_step_counter_pos] = BaseElement::ZERO;
         */
        
           
        // Now we've filled data_size + 3 positions 
        
        // fill_selector_info(&mut inputs, 0u128, trace_128, data_size);
        // To prove fft(omega, coeff: &[BaseElement], degree) = output: &[BaseElement]
        trace.fill(
            |state| {
                for i in 0..inputs.len() {
                    state[i] = inputs[i]
                }
                state[num_fft_inputs] = BaseElement::ONE;
                fill_selector_info(state, 0, num_fft_inputs);
            },
            |step, state| {
                // if step < self.num_main_trace_rows - 1 {
                let next_omega = omega.exp((num_fft_inputs/(1 << (step))).try_into().unwrap());
                state[num_fft_inputs] = next_omega;
                apply_iterative_fft_layer(step, state, num_fft_inputs, omega)
            
                // }
                // else {
                    // apply_simple_copy(state);
                // }
            }
        );
        let mut last_trace_row = vec![BaseElement::ONE; trace_width];
        trace.read_row_into(get_results_row_idx(self.num_main_trace_rows), &mut last_trace_row);
        for (col, res) in result.iter().take(num_fft_inputs).enumerate() {
            debug_assert_eq!(trace.get(col, get_results_row_idx(self.num_main_trace_rows)), *res, "Column {}", col);
        }
        // println!("Last trace row = {:?}", last_trace_row.clone());
        // println!("Result = {:?}", result.clone());
        println!("Assertions passed");

        trace
    }

    // fn get_initial_coeffs(&self, trace: &TraceTable<BaseElement>) -> Vec<BaseElement> {
    //     (0..self.num_fft_inputs).map(|x| trace.get(x, 0)).collect()
    // }

    // pub(crate) fn get_final_outputs(&self, trace: &TraceTable<BaseElement>) -> Vec<BaseElement> {
    //     (0..self.degree).map(|x| trace.get(x, trace.length() - 1)).collect()
    // }

    // fn get_omega(&self, trace: &TraceTable<BaseElement>) -> BaseElement {
    //     trace.get(self.degree, 0)
    // }

}




impl Prover for FFTProver {
    type BaseField = BaseElement;
    type Air = FFTAir;
    type Trace = TraceTable<BaseElement>;

    fn get_pub_inputs(&self, trace: &Self::Trace) -> PublicInputs {
        let last_fft_state = get_results_row_idx(self.num_main_trace_rows);
        let num_inputs = get_num_fft_inputs(self.num_main_trace_rows);
        let mut fft_input_vec = vec![BaseElement::ONE; trace.width()];
        trace.read_row_into(0, &mut fft_input_vec);
        let mut fft_output_vec = vec![BaseElement::ONE; trace.width()];
        trace.read_row_into(last_fft_state, &mut fft_output_vec);
        PublicInputs {
            num_inputs,
            fft_inputs: fft_input_vec[..num_inputs].to_vec(),
            result: fft_output_vec[..num_inputs].to_vec(),
        }
    }
    fn options(&self) -> &ProofOptions {
        &self.options
    }
}




// TRANSITION FUNCTION
// ================================================================================================
fn apply_iterative_fft_layer(step: usize, state: &mut [BaseElement], num_fft_inputs: usize, omega: BaseElement) {
    // state[num_fft_inputs + 2] = state[num_fft_inputs + 2] + BaseElement::ONE;
    fill_selector_info(state, step + 1, num_fft_inputs);
    // Swapping for the butterfly network
    if step == 0 {
        // let log_degree: usize = log2(num_fft_inputs).try_into().unwrap();
        // for i in 0..num_fft_inputs {
        //     let rev = bit_reverse(i, log_degree);
        //     if i < rev {
        //         swap(i, rev, state);
        //     }
        // }
        apply_bit_rev_copy_permutation(state, num_fft_inputs);
    }
    else {
        apply_fft_calculation(state, step, omega, num_fft_inputs);
        // let curr_omega = state[data_size];

        // // actual fft
        // let jump = 1 << step;
        // let mut counter = 0;
        // while counter < data_size {
        //     let mut curr_pow = BaseElement::ONE;
        //     for j in 0..(1 << (step - 1)) {
        //         let u = state[counter + j];
        //         let v = state[counter + j + (1 << (step - 1))] * curr_pow;
        //         state[counter + j] = u + v;
        //         state[counter + j + (1 << (step - 1))] = u - v;
        //         curr_pow = curr_pow * curr_omega;
        //     }
        //     counter = counter + jump;
        // }
        
    }
    // let step_u128: u128 = step.try_into().unwrap();
    // fill_selector_info(state, step_u128+1, trace_length, data_size);
    // Calculate the curr_omega to be used in the next step
    

}

// Applies the FFT calculation for the step-th step with the appropriate omega
fn apply_fft_calculation(state: &mut [BaseElement], step: usize, omega: BaseElement, fft_size: usize) {
    let fft_size_u128: u128 = fft_size.try_into().unwrap();
    let m = 1 << step;
    let m_u128: u128 = m.try_into().unwrap();
    let mut omegas = Vec::<BaseElement>::new();
    let mut power_of_omega = BaseElement::ONE;
    let local_omega = omega.exp(fft_size_u128 / m_u128);
    let jump = (1 << step) / 2;
    let num_ranges = fft_size / m;
    for _ in 0..m {
        omegas.push(power_of_omega);
        power_of_omega *= local_omega;
    }
    for k in 0..num_ranges {
        let start_of_range = k * 2 * jump;
        for j in 0..jump {
            let curr_omega = omegas[j];
            let u = state[start_of_range + j];
            let v = state[start_of_range + j + jump] * curr_omega;
            state[start_of_range + j] = u + v;
            state[start_of_range + j + jump] = u - v;
        }
    }
}


fn apply_bit_rev_copy_permutation(state: &mut [BaseElement], num_fft_inputs: usize) {
    let fft_size = state.len();
    let log_fft_size = log2(num_fft_inputs);
    let num_bits: usize = log_fft_size.try_into().unwrap();
    let mut next_state = vec![BaseElement::ZERO; fft_size];
    for i in 0..fft_size {
        next_state[i] = state[i];
    }
    for i in 0..num_fft_inputs {
        next_state[bit_reverse(i, num_bits)] = state[i];
    }
    state[..fft_size].copy_from_slice(&next_state[..fft_size]);
}

fn apply_simple_copy(state: &mut [BaseElement]) {
    let fft_width = state.len();
    let mut next_state = vec![BaseElement::ZERO; fft_width];
    for i in 0..fft_width {
        next_state[i] = state[i];
    }
    state[..fft_width].copy_from_slice(&next_state[..fft_width]);
}

// // HELPER FUNCTIONS
// // ================================================================================================

fn fill_selector_info(state: &mut [BaseElement], row_number: usize, num_fft_inputs: usize) {
    let power_of_two_pos = get_power_of_two_pos(num_fft_inputs);
    if row_number == 0 {
        state[power_of_two_pos] = BaseElement::from(2u64);
    }
    // else if row_number == 1 {
    //     state[power_of_two_pos] = BaseElement::from(2u64);
    // }
    else {
        state[power_of_two_pos] = BaseElement::from(2u64) * state[power_of_two_pos];
    }
    let log_num_fft_inputs: usize = log2(num_fft_inputs).try_into().unwrap();
    for i in 0..log_num_fft_inputs+1 {
        let selector_pos = get_selector_pos(i, num_fft_inputs);
        if i == row_number {
            state[selector_pos] = BaseElement::ONE;
        }
        else {
            state[selector_pos] = BaseElement::ZERO;
        }
    }
    let rev_counter_pos = get_rev_counter_pos(num_fft_inputs, log_num_fft_inputs);
    let rev_counter_inv_pos = get_rev_counter_inv_pos(num_fft_inputs, log_num_fft_inputs);
    if row_number == 0 {
        let x = BaseElement::from(get_num_main_trace_rows_u64(num_fft_inputs));
        let f_x = x.inv();
        state[rev_counter_pos] = x;
        state[rev_counter_inv_pos] = f_x;

    }
    else {
        if state[rev_counter_pos] == BaseElement::ZERO {
            state[rev_counter_pos] = BaseElement::ZERO;
            state[rev_counter_inv_pos] = BaseElement::ONE;
        }
        else {
            let x = state[rev_counter_pos] - BaseElement::ONE;
            let f_x = {if x == BaseElement::ZERO { BaseElement::ONE } else { x.inv()}};
            state[rev_counter_pos] = x;
            state[rev_counter_inv_pos] = f_x;
        }
    }
    // for i in 0..trace_128 {
    //     let i_group_elt = {
    //         if i == step {
    //             BaseElement::ONE
    //         }
    //         else {
    //             BaseElement::ZERO
    //         }
    //     };
    //     state[get_selector_pos(i.try_into().unwrap(), degree)] = i_group_elt;
    // }
}

// fn fill_selector_info_old(state: &mut [BaseElement], step: BaseElement, trace_128: u128, degree: usize) {
//     for i in 0..trace_128 {
//         let i_group_elt = BaseElement::from(i) - step;
//         let i_inv_try = {
//             if i_group_elt == BaseElement::ZERO {
//                 BaseElement::ONE
//             }
//             else {
//                 i_group_elt.inv()
//             }
//         };
//         let selector_bit = BaseElement::ONE - (i_group_elt * i_inv_try);
//         let i_usize: usize = i.try_into().unwrap();
//         state[get_count_diff_pos(i_usize, degree)] = i_group_elt;
//         state[get_inv_pos(i_usize, degree)] = i_inv_try;
//         state[get_selector_pos(i_usize, degree)] = selector_bit;

//     }
// }




fn swap(pos1: usize, pos2: usize, state: &mut [BaseElement]) {
    let temp = state[pos1];
    state[pos1] = state[pos2];
    state[pos2] = temp;
}

pub(crate) fn get_num_trace_rows(num_fft_inputs: usize) -> usize {
    let log_num_fft_terms: usize = log2(num_fft_inputs).try_into().unwrap();
    let main_trace_rows = log_num_fft_terms + 2;
    main_trace_rows.next_power_of_two()
}

pub(crate) fn get_num_main_trace_rows(num_fft_inputs: usize) -> usize {
    let log_num_fft_terms: usize = log2(num_fft_inputs).try_into().unwrap();
    log_num_fft_terms + 2
}

pub(crate) fn get_num_main_trace_rows_u64(num_fft_inputs: usize) -> u64 {
    let log_num_fft_terms: u64 = log2(num_fft_inputs).try_into().unwrap();
    log_num_fft_terms + 2
}



pub(crate) fn get_num_cols(num_fft_inputs: usize) -> usize {
    let log_num_fft_terms: usize = log2(num_fft_inputs).try_into().unwrap();
    // the first num_fft_inputs are for keeping the actual values at eachs step
    // the next value is for keeping the local omegas
    // Then, we store log_num_fft_terms + 1 bits which help select the function to apply
    // Finally, the additional position is to keep the power of 2 represented by the aforementioned bits
    num_fft_inputs + 1 + 1 + (log_num_fft_terms + 1) + 2
}

pub(crate) fn get_num_fft_inputs(num_rows: usize) -> usize {
    let log_num_inputs: usize = num_rows - 2;
    1 << log_num_inputs
}

pub(crate) fn get_results_row_idx(num_main_rows: usize) -> usize {
    num_main_rows - 1
}

pub(crate) fn get_power_of_two_pos(num_fft_inputs: usize) -> usize {
    num_fft_inputs + 1
}

fn get_selector_pos(i: usize, num_fft_inputs: usize) -> usize {
    num_fft_inputs + 2 + i
}

pub(crate) fn get_rev_counter_pos(num_fft_inputs: usize, log_num_fft_terms: usize) -> usize {
    num_fft_inputs + 1 + 1 + log_num_fft_terms + 1
}

pub(crate) fn get_rev_counter_inv_pos(num_fft_inputs: usize, log_num_fft_terms: usize) -> usize {
    num_fft_inputs + 1 + 1 + log_num_fft_terms + 1 + 1
}



// fn get_selector_bit_pos(i: usize, degree: usize) -> usize {
//     degree + 3 + i
// }

// fn get_count_diff_pos(i: usize, degree: usize) -> usize {
//     degree + 3 + 3*i
// }

// fn get_inv_pos(i: usize, degree: usize) -> usize {
//     degree + 3 + 3*i + 1
// }


