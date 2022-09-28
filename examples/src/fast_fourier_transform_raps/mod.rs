// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::{Example, ExampleOptions};
use log::debug;
use rand_utils::rand_array;
use core::num;
use std::{time::Instant, collections::VecDeque, convert::TryInto};
use winterfell::{
    math::{fields::f128::BaseElement, log2, ExtensionOf, FieldElement, fft, StarkField},
    ProofOptions, Prover, StarkProof, Trace, VerifierError,
};

mod custom_trace_table;
pub use custom_trace_table::FFTTraceTable;

use super::rescue::rescue::{self, STATE_WIDTH};

mod air;
use air::{PublicInputs, FFTRapsAir};

mod prover;
use prover::FFTRapsProver;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

const CYCLE_LENGTH: usize = 16;
const NUM_HASH_ROUNDS: usize = 14;


// RESCUE SPLIT HASH CHAIN EXAMPLE
// ================================================================================================

pub fn get_example(options: ExampleOptions, num_fft_inputs: usize) -> Box<dyn Example> {
    Box::new(FFTRapsExample::new(
        num_fft_inputs,
        options.to_proof_options(42, 8),
    ))
}

pub struct FFTRapsExample {
    options: ProofOptions,
    omega: BaseElement,
    num_fft_inputs: usize,
    fft_inputs: Vec<BaseElement>,
    result: Vec<BaseElement>,
}

impl FFTRapsExample {
    pub fn new(num_fft_inputs: usize, options: ProofOptions) -> FFTRapsExample {
        assert!(
            num_fft_inputs.is_power_of_two(),
            "number of inputs for fft must a power of 2"
        );
        // assert!(num_fft_inputs > 128, "number of inputs must be at least 128");

        let mut fft_inputs = vec![BaseElement::ZERO; num_fft_inputs as usize];
        for internal_seed in fft_inputs.iter_mut() {
            *internal_seed = rand_array::<_, 1>()[0];
        }

        // compute the sequence of hashes using external implementation of Rescue hash
        let now = Instant::now();
        let omega = BaseElement::get_root_of_unity(num_fft_inputs.try_into().unwrap());
        let result = simple_iterative_fft(fft_inputs.clone(), omega);
        debug!(
            "Computed fft of {} inputs in {} ms",
            num_fft_inputs,
            now.elapsed().as_millis(),
        );

        

        FFTRapsExample {
            options,
            omega,
            num_fft_inputs,
            fft_inputs,
            result,
        }
    }
}

// EXAMPLE IMPLEMENTATION
// ================================================================================================

impl Example for FFTRapsExample {
    fn prove(&self) -> StarkProof {
        // generate the execution trace
        debug!(
            "Generating proof for computing an FFT on {} inputs\n\
            ---------------------",
            self.num_fft_inputs
        );

        // create a prover
        let prover = FFTRapsProver::new(self.options.clone());

        let fft_in = self.fft_inputs.clone();

        // generate the execution trace
        let now = Instant::now();
        let trace = prover.build_trace(self.omega, &self.fft_inputs, &self.result);
        let trace_length = trace.length();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace.width(),
            log2(trace_length),
            now.elapsed().as_millis()
        );
        let mut last_trace_col = vec![BaseElement::ONE; trace_length];
        trace.read_col_into(FFTRapsProver::get_results_col_idx(self.num_fft_inputs), &mut last_trace_col);
        // generate the proof
        prover.prove(trace).unwrap()
    }

    fn verify(&self, proof: StarkProof) -> Result<(), VerifierError> {
        let pub_inputs = PublicInputs {
            result: self.result.clone(),
            num_inputs: self.num_fft_inputs,
            fft_inputs: self.fft_inputs.clone(),
        };
        winterfell::verify::<FFTRapsAir>(proof, pub_inputs)
    }

    fn verify_with_wrong_inputs(&self, proof: StarkProof) -> Result<(), VerifierError> {
        // let pub_inputs = PublicInputs {
        //     result: [self.result[1], self.result[0]],
        // };
        // winterfell::verify::<FFTRapsAir>(proof, pub_inputs)
        // unimplemented!()
        Ok(())
    }
}

// HELPER FUNCTIONS
// ================================================================================================

// Implements a simple iterative FFT using the Cooley-Tukey algorithm from 
// https://en.wikipedia.org/wiki/Cooley–Tukey_FFT_algorithm#Data_reordering,_bit_reversal,_and_in-place_algorithms
fn simple_iterative_fft(input_array: Vec<BaseElement>, omega: BaseElement) -> Vec<BaseElement> {
    let mut output_arr = bit_reverse_copy(input_array.clone());
    let fft_size = input_array.len();
    let log_fft_size = log2(fft_size);
    let num_steps: usize = log_fft_size.try_into().unwrap();
    let fft_size_u128: u128 = fft_size.try_into().unwrap();
    for step in 1..num_steps+1 {
        let m = 1 << step;
        let omega_pow = fft_size_u128 / m;
        let local_factor = omega.exp(omega_pow); 
        let k_upperbound: usize = (fft_size_u128/m).try_into().unwrap();
        let jump: usize = (m/2).try_into().unwrap();
        for k in 0..k_upperbound {
            let mut omega_curr = BaseElement::ONE;
            let start_pos = k*jump*2;
            for j in 0..jump {
                let u = output_arr[start_pos+j];
                let v = omega_curr * output_arr[start_pos+j+jump];
                output_arr[start_pos+j] = u + v;
                output_arr[start_pos+j+jump] = u - v;
                omega_curr = omega_curr * local_factor;
                
            }
        }
    }
    output_arr
}

fn bit_reverse_copy(input_array: Vec<BaseElement>) -> Vec<BaseElement> {
    let mut output_arr = input_array.clone();
    let fft_size = input_array.len();
    let log_fft_size = log2(fft_size);
    let num_bits: usize = log_fft_size.try_into().unwrap();
    for i in 0..fft_size {
        output_arr[bit_reverse(i, num_bits)] = input_array[i];
    }
    output_arr
}





fn bit_reverse(input_int: usize, num_bits: usize) -> usize {
    let mut output_int = 0;
    let mut input_copy = input_int;
    for _ in 0..num_bits {
        output_int <<= 1;
        output_int |= input_copy & 1;
        input_copy >>= 1;
    }
    return output_int;
}

fn apply_bit_rev_copy_permutation(state: &mut [BaseElement]) {
    let fft_size = state.len();
    let log_fft_size = log2(fft_size);
    let num_bits: usize = log_fft_size.try_into().unwrap();
    let mut next_state = vec![BaseElement::ZERO; fft_size];
    for i in 0..fft_size {
        next_state[bit_reverse(i, num_bits)] = state[i];
    }
    for i in 0..fft_size {
        state[i] = next_state[i];
    }
}

fn apply_fft_permutation(state: &mut [BaseElement], step: usize) {
    assert!(step % 2 == 0, "Only even steps have permuations");
    let fft_size = state.len();
    let jump = (1 << (step/2 + 1))/2;
    let num_ranges = fft_size / (2*jump);
    let mut next_state = vec![BaseElement::ZERO; fft_size];
    for k in 0..num_ranges {
        let start_of_range = k * 2 * jump;
        for j in 0..jump {
            next_state[start_of_range + 2*j] = state[start_of_range + j];
            next_state[start_of_range + 2*j + 1] = state[start_of_range + j + jump];
        }
    }
    for i in 0..fft_size {
        state[i] = next_state[i];
    }
}

fn apply_fft_inv_permutation(state: &mut [BaseElement], step: usize) {
    assert!(step != 0, "Only non-zero steps have inv permuations");
    assert!(step % 2 == 0, "Only even steps have inv permuations");
    let step_prev = step - 2;
    let fft_size = state.len();
    let jump = (1 << (step_prev/2 + 1))/2;
    let num_ranges = fft_size / (2*jump);
    let mut next_state = vec![BaseElement::ZERO; fft_size];
    for k in 0..num_ranges {
        let start_of_range = k * 2 * jump;
        
        for j in 0..jump {
            next_state[start_of_range + j] = state[start_of_range + 2*j];
            next_state[start_of_range + j + jump] = state[start_of_range + 2*j + 1];
        }
    }
    
    for i in 0..fft_size {
        state[i] = next_state[i];
    }  
}

fn fill_fft_indices(state: &mut [BaseElement]) {
    let fft_size = state.len();
    let mut count = 0u128;
    let mut count_usize = 0;
    while count_usize < fft_size {
        state[count_usize] = BaseElement::from(count);
        count_usize += 1;
        count += 1;
    }
}

fn apply_fft_calculation(state: &mut [BaseElement], step: usize, omega: BaseElement) {
    assert!(step % 2 == 1, "Only odd steps have computation steps");
    let fft_size = state.len();
    let fft_size_u128: u128 = fft_size.try_into().unwrap();
    let m = 1 << ((step + 1)/2);
    let m_u128: u128 = m.try_into().unwrap();
    let mut omegas = Vec::<BaseElement>::new();
    let mut power_of_omega = BaseElement::ONE;
    let local_omega = omega.exp(fft_size_u128/m_u128);
    for _ in 0..m {
        omegas.push(power_of_omega);
        power_of_omega *= local_omega;
    }
    for i in 0..fft_size/2 {
        let curr_omega = omegas[i % (m/2)];
        let u = state[2*i];
        let v = state[2*i+1] * curr_omega;
        state[2*i] = u + v;
        state[2*i + 1] = u - v;
    }
}



/////// Tests for helpers

#[test]
fn apply_fft_permutation_test_size_4() {
    let mut state = [BaseElement::new(0), BaseElement::new(1), BaseElement::new(2), BaseElement::new(3)];
    let expected_output_state = [BaseElement::new(0), BaseElement::new(2), BaseElement::new(1), BaseElement::new(3)];
    apply_fft_permutation(&mut state, 0);
    for j in 0..4 {
        assert_eq!(state[j], expected_output_state[j], 
            "Output state {} is {:?}, expexted {:?}", 
            j, state[j], expected_output_state[j]);
    }
}

#[test]
fn apply_fft_permutation_test_size_8() {
    let mut state = [BaseElement::new(0), BaseElement::new(1), 
                                        BaseElement::new(2), BaseElement::new(3),
                                        BaseElement::new(4), BaseElement::new(5),
                                        BaseElement::new(6), BaseElement::new(7)
                                    ];
    let expected_output_state_step_0 = [BaseElement::new(0), BaseElement::new(4), 
                                                            BaseElement::new(1), BaseElement::new(5),
                                                            BaseElement::new(2), BaseElement::new(6),
                                                            BaseElement::new(3), BaseElement::new(7)
                                                        ];
    apply_fft_permutation(&mut state, 0);
    for j in 0..4 {
        assert_eq!(state[j], expected_output_state_step_0[j], 
            "Output state {} is {:?}, expexted {:?}", 
            j, state[j], expected_output_state_step_0[j]);
    }
    let mut new_state = [BaseElement::new(0), BaseElement::new(1), 
                                        BaseElement::new(2), BaseElement::new(3),
                                        BaseElement::new(4), BaseElement::new(5),
                                        BaseElement::new(6), BaseElement::new(7)
                                    ];
    let expected_output_state_step_1 = [BaseElement::new(0), BaseElement::new(2), 
                                                            BaseElement::new(1), BaseElement::new(3),
                                                            BaseElement::new(4), BaseElement::new(6),
                                                            BaseElement::new(5), BaseElement::new(7)
                                                        ];
    apply_fft_permutation(&mut new_state, 2);
    for j in 0..4 {
        assert_eq!(new_state[j], expected_output_state_step_1[j], 
            "Output state {} is {:?}, expexted {:?}", 
            j, new_state[j], expected_output_state_step_1[j]);
    }

}