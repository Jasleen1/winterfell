// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::{Example, ExampleOptions};
use log::debug;
use rand_utils::rand_array;
use core::num;
use std::{time::Instant, collections::VecDeque};
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
const TRACE_WIDTH: usize = 4 * 2;

// RESCUE SPLIT HASH CHAIN EXAMPLE
// ================================================================================================

pub fn get_example(options: ExampleOptions, num_fft_inputs: usize) -> Box<dyn Example> {
    Box::new(FFTRapsExample::new(
        num_fft_inputs,
        options.to_proof_options(42, 4),
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
        assert!(num_fft_inputs > 128, "number of inputs must be at least 128");

        let mut fft_inputs = vec![BaseElement::ZERO; num_fft_inputs as usize];
        for internal_seed in fft_inputs.iter_mut() {
            *internal_seed = rand_array::<_, 1>()[0];
        }

        // compute the sequence of hashes using external implementation of Rescue hash
        let now = Instant::now();
        // TODO #1 write and test a plain fft computation here
        let result = vec![BaseElement::ZERO; num_fft_inputs as usize];
        debug!(
            "Computed two permuted chains of {} Rescue hashes in {} ms",
            num_fft_inputs,
            now.elapsed().as_millis(),
        );

        let omega = BaseElement::get_root_of_unity(num_fft_inputs.try_into().unwrap());

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
        unimplemented!()
    }
}

// HELPER FUNCTIONS
// ================================================================================================



fn apply_fft_permutation(state: &mut [BaseElement], step: usize) {
    assert!(step % 2 == 0, "Only even steps have permuations");
    let fft_size = state.len();
    let jump = fft_size/(1 << (step/2 + 1));
    let num_ranges = 1 << step/2;
    let mut next_state = vec![BaseElement::ZERO; fft_size];
    for i in 0..num_ranges {
        let start_of_range = i * 2 * jump;
        for j in 0..jump/2+1 {
            next_state[start_of_range + 2*j] = state[start_of_range + j];
            next_state[start_of_range + 2*j + 1] = state[start_of_range + j + jump];
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
    let power_mod = 1 << ((step + 1)/2);
    let mut omegas = Vec::<BaseElement>::new();
    let mut power_of_omega = BaseElement::ONE;
    for _ in 0..power_mod {
        omegas.push(power_of_omega);
        power_of_omega *= omega;
    }
    for i in 0..fft_size/2 {
        let curr_omega = omegas[step % power_mod];
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