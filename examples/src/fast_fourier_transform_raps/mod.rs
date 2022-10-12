// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::{
    fast_fourier_transform_raps::prover::get_results_col_idx,
    utils::fast_fourier_transform::bit_reverse, Example, ExampleOptions,
};
use core::num;
use log::debug;
use rand_utils::rand_array;
use std::{collections::VecDeque, convert::TryInto, time::Instant};
use winterfell::{
    math::{fft, fields::f128::BaseElement, log2, ExtensionOf, FieldElement, StarkField},
    ProofOptions, Prover, StarkProof, Trace, VerifierError,
};

use crate::utils::fast_fourier_transform::simple_iterative_fft;

mod custom_trace_table;
pub use custom_trace_table::FFTTraceTable;

use super::rescue::rescue::{self, STATE_WIDTH};

mod air;
use air::{FFTRapsAir, PublicInputs};

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
        trace.read_col_into(
            get_results_col_idx(self.num_fft_inputs),
            &mut last_trace_col,
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
        // unimplemented!()
        Ok(())
    }
}

// HELPER FUNCTIONS
// ================================================================================================

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
    assert!(step % 3 == 2, "Only 2 (mod 3) steps have permuations");
    let fft_size = state.len();
    let perm_step = (step + 1) / 3 + 1;
    let jump = (1 << perm_step) / 2;
    let num_ranges = fft_size / (2 * jump);
    let mut next_state = vec![BaseElement::ZERO; fft_size];
    for k in 0..num_ranges {
        let start_of_range = k * 2 * jump;
        for j in 0..jump {
            next_state[start_of_range + 2 * j] = state[start_of_range + j];
            next_state[start_of_range + 2 * j + 1] = state[start_of_range + j + jump];
        }
    }
    // if perm_step == 2 {
    //     println!("Original for step 1 = {:?}", state);
    //     println!("perm for step 1 = {:?}", next_state);
    // }
    for i in 0..fft_size {
        state[i] = next_state[i];
    }
}

fn apply_fft_inv_permutation(state: &mut [BaseElement], step: usize) {
    assert!(step != 0, "Only non-zero steps have inv permuations");
    assert!(
        step % 3 == 1,
        "Only steps of the form 1 (mod 3) have inv permuations"
    );
    let step_prev = step - 2;
    let fft_size = state.len();
    let perm_step = (step_prev + 1) / 3 + 1;
    let jump = (1 << perm_step) / 2;
    let num_ranges = fft_size / (2 * jump);
    let mut next_state = vec![BaseElement::ZERO; fft_size];
    for k in 0..num_ranges {
        let start_of_range = k * 2 * jump;

        for j in 0..jump {
            next_state[start_of_range + j] = state[start_of_range + 2 * j];
            next_state[start_of_range + j + jump] = state[start_of_range + 2 * j + 1];
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
    assert!(
        (step == 1 || step % 3 == 0),
        "Only step 1 or every 3rd step has computation steps"
    );
    let fft_size = state.len();
    let fft_size_u128: u128 = fft_size.try_into().unwrap();
    let mut m = 1 << ((step + 1) / 2);
    if step % 3 == 0 {
        m = 1 << ((step / 3) + 1);
    }
    let m_u128: u128 = m.try_into().unwrap();
    let mut omegas = Vec::<BaseElement>::new();
    let mut power_of_omega = BaseElement::ONE;
    let local_omega = omega.exp(fft_size_u128 / m_u128);
    for _ in 0..m {
        omegas.push(power_of_omega);
        power_of_omega *= local_omega;
    }
    for i in 0..fft_size / 2 {
        let curr_omega = omegas[i % (m / 2)];
        let u = state[2 * i];
        let v = state[2 * i + 1] * curr_omega;
        state[2 * i] = u + v;
        state[2 * i + 1] = u - v;
    }
}

/// This function maps an integer j -> new_location(j) after applying the
/// permutation for the given fft step. Note that step here ranges from
/// 1-log(fft_size) (both included)
fn get_fft_permutation_locs(fft_size: usize, step: usize) -> Vec<usize> {
    assert!(step >= 1, "Step number must be at least 1");
    assert!(
        1 << step <= fft_size,
        "Step number is upper bounded by log(fft_size"
    );
    let jump = (1 << step) / 2;
    let num_ranges = fft_size / (2 * jump);
    let mut perm_locs = vec![0; fft_size];
    for k in 0..num_ranges {
        let start_of_range = k * 2 * jump;
        for j in 0..jump {
            perm_locs[start_of_range + j] = start_of_range + 2 * j;
            perm_locs[start_of_range + j + jump] = start_of_range + 2 * j + 1;
        }
    }
    perm_locs
}

/// This function maps an integer new_location(j) -> j after applying the
/// permutation for the given fft step. Note that step here ranges from
/// 1-log(fft_size) (both included)
fn get_fft_inv_permutation_locs(fft_size: usize, step: usize) -> Vec<usize> {
    assert!(step >= 1, "Step number must be at least 1");
    assert!(
        1 << step <= fft_size,
        "Step number is upper bounded by log(fft_size"
    );
    let jump = (1 << step) / 2;
    let num_ranges = fft_size / (2 * jump);
    let mut perm_locs = vec![0; fft_size];
    for k in 0..num_ranges {
        let start_of_range = k * 2 * jump;
        for j in 0..jump {
            perm_locs[start_of_range + 2 * j] = start_of_range + j;
            perm_locs[start_of_range + 2 * j + 1] = start_of_range + j + jump;
        }
    }
    perm_locs
}

/////// Tests for helpers

#[test]
fn apply_fft_permutation_test_size_4() {
    let mut state = [
        BaseElement::new(0),
        BaseElement::new(1),
        BaseElement::new(2),
        BaseElement::new(3),
    ];
    let expected_output_state = [
        BaseElement::new(0),
        BaseElement::new(2),
        BaseElement::new(1),
        BaseElement::new(3),
    ];
    apply_fft_permutation(&mut state, 0);
    for j in 0..4 {
        assert_eq!(
            state[j], expected_output_state[j],
            "Output state {} is {:?}, expexted {:?}",
            j, state[j], expected_output_state[j]
        );
    }
}

#[test]
fn apply_fft_permutation_test_size_8() {
    let mut state = [
        BaseElement::new(0),
        BaseElement::new(1),
        BaseElement::new(2),
        BaseElement::new(3),
        BaseElement::new(4),
        BaseElement::new(5),
        BaseElement::new(6),
        BaseElement::new(7),
    ];
    let expected_output_state_step_0 = [
        BaseElement::new(0),
        BaseElement::new(4),
        BaseElement::new(1),
        BaseElement::new(5),
        BaseElement::new(2),
        BaseElement::new(6),
        BaseElement::new(3),
        BaseElement::new(7),
    ];
    apply_fft_permutation(&mut state, 0);
    for j in 0..4 {
        assert_eq!(
            state[j], expected_output_state_step_0[j],
            "Output state {} is {:?}, expexted {:?}",
            j, state[j], expected_output_state_step_0[j]
        );
    }
    let mut new_state = [
        BaseElement::new(0),
        BaseElement::new(1),
        BaseElement::new(2),
        BaseElement::new(3),
        BaseElement::new(4),
        BaseElement::new(5),
        BaseElement::new(6),
        BaseElement::new(7),
    ];
    let expected_output_state_step_1 = [
        BaseElement::new(0),
        BaseElement::new(2),
        BaseElement::new(1),
        BaseElement::new(3),
        BaseElement::new(4),
        BaseElement::new(6),
        BaseElement::new(5),
        BaseElement::new(7),
    ];
    apply_fft_permutation(&mut new_state, 2);
    for j in 0..4 {
        assert_eq!(
            new_state[j], expected_output_state_step_1[j],
            "Output state {} is {:?}, expexted {:?}",
            j, new_state[j], expected_output_state_step_1[j]
        );
    }
}
