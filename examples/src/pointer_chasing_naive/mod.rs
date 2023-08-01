// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::{Blake3_192, Blake3_256, Example, ExampleOptions, HashFunction, Sha3_256};
use core::marker::PhantomData;
use log::debug;
use rand_utils::rand_array;
use std::{num, time::Instant};
use winterfell::{
    crypto::{DefaultRandomCoin, ElementHasher},
    math::{fields::f128::BaseElement, ExtensionOf, FieldElement},
    ProofOptions, Prover, StarkProof, Trace, VerifierError,
};

use super::rescue::rescue::{self, STATE_WIDTH};

mod air;
use air::{PointerChasingComponentAir, PublicInputs};

mod prover;
use prover::PointerChasingNaiveProver;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

const CYCLE_LENGTH: usize = 16;
const NUM_HASH_ROUNDS: usize = 14;
const TRACE_WIDTH: usize = 4 * 2;

// RESCUE SPLIT HASH CHAIN EXAMPLE
// ================================================================================================

pub fn get_example(
    options: &ExampleOptions,
    num_locs: usize,
    num_steps: usize,
) -> Result<Box<dyn Example>, String> {
    let (options, hash_fn) = options.to_proof_options(42, 4);

    match hash_fn {
        HashFunction::Blake3_192 => Ok(Box::new(PointerChasingNaiveExample::<Blake3_192>::new(
            num_locs, num_steps, options,
        ))),
        HashFunction::Blake3_256 => Ok(Box::new(PointerChasingNaiveExample::<Blake3_256>::new(
            num_locs, num_steps, options,
        ))),
        HashFunction::Sha3_256 => Ok(Box::new(PointerChasingNaiveExample::<Sha3_256>::new(
            num_locs, num_steps, options,
        ))),
        _ => Err("The specified hash function cannot be used with this example.".to_string()),
    }
}

pub struct PointerChasingNaiveExample<H: ElementHasher> {
    options: ProofOptions,
    num_locs: usize,
    num_steps: usize,
    inputs: [usize; 2],
    result: BaseElement,
    _hasher: PhantomData<H>,
}

impl<H: ElementHasher> PointerChasingNaiveExample<H> {
    pub fn new(num_steps: usize, num_locs: usize, options: ProofOptions) -> Self {
        assert!(
            num_locs.is_power_of_two(),
            "number of locations must a power of 2"
        );
        assert!(
            num_steps.is_power_of_two(),
            "number of RAM steps must a power of 2"
        );
        assert!(num_locs <= num_steps, "Want more steps than locations");

        // let mut seeds: [BaseElement; _] = [BaseElement::ZERO; 2];
        // seeds = rand_array();
        let inputs = [1, 2];

        // compute the sequence of hashes using external implementation of Rescue hash
        let now = Instant::now();
        let result = usize_to_base_elt(plain_pointer_chase(num_locs, num_steps, inputs));
        debug!(
            "Computed result of {} steps with {} locs in {} ms",
            num_steps,
            num_locs,
            now.elapsed().as_millis(),
        );
        println!("Plaintext result = {}", result);
        PointerChasingNaiveExample {
            options,
            num_locs,
            num_steps,
            inputs,
            result,
            _hasher: PhantomData,
        }
    }
}

// EXAMPLE IMPLEMENTATION
// ================================================================================================

impl<H: ElementHasher> Example for PointerChasingNaiveExample<H>
where
    H: ElementHasher<BaseField = BaseElement>,
{
    fn prove(&self) -> StarkProof {
        // generate the execution trace
        debug!(
            "Generating proof for {} calculations on a memory of size {}\n\
            ---------------------",
            self.num_steps, self.num_locs
        );

        // create a prover
        let mut prover = PointerChasingNaiveProver::<H>::new(
            self.options.clone(),
            self.num_locs,
            self.num_steps,
        );

        // generate the execution trace
        let now = Instant::now();
        let trace = prover.build_trace(self.inputs[0], self.inputs[1]);
        let trace_length = trace.length();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace.width(),
            trace_length.ilog2(),
            now.elapsed().as_millis()
        );

        // generate the proof
        prover.prove(trace).unwrap()
    }

    fn verify(&self, proof: StarkProof) -> Result<(), VerifierError> {
        let pub_inputs = PublicInputs {
            result: self.result,
            num_locs: self.num_locs,
            num_steps: self.num_steps,
        };
        winterfell::verify::<PointerChasingComponentAir, H, DefaultRandomCoin<H>>(proof, pub_inputs)
    }

    fn verify_with_wrong_inputs(&self, proof: StarkProof) -> Result<(), VerifierError> {
        let pub_inputs = PublicInputs {
            result: self.result + BaseElement::ONE,
            num_locs: self.num_locs,
            num_steps: self.num_steps,
        };
        winterfell::verify::<PointerChasingComponentAir, H, DefaultRandomCoin<H>>(proof, pub_inputs)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn absorb(state: &mut [BaseElement; STATE_WIDTH], values: &[BaseElement; 2]) {
    state[0] += values[0];
    state[1] += values[1];
    for i in 0..NUM_HASH_ROUNDS {
        rescue::apply_round(state, i);
    }
}

fn compute_permuted_hash_chains(
    seeds: &[[BaseElement; 2]],
    permuted_seeds: &[[BaseElement; 2]],
) -> [[BaseElement; 2]; 2] {
    let mut state = [BaseElement::ZERO; STATE_WIDTH];
    let mut permuted_state = [BaseElement::ZERO; STATE_WIDTH];

    // Start the hash chain
    for (seed, permuted_seed) in seeds.iter().zip(permuted_seeds) {
        absorb(&mut state, seed);
        absorb(&mut permuted_state, permuted_seed);
    }

    [[state[0], state[1]], [permuted_state[0], permuted_state[1]]]
}

fn apply_rescue_round_parallel(multi_state: &mut [BaseElement], step: usize) {
    debug_assert_eq!(multi_state.len() % STATE_WIDTH, 0);

    for state in multi_state.chunks_mut(STATE_WIDTH) {
        rescue::apply_round(state, step)
    }
}

fn plain_pointer_chase(num_locs: usize, num_steps: usize, inputs: [usize; 2]) -> usize {
    let mut running_state = (0..num_locs).collect::<Vec<usize>>();
    running_state[0] = running_state[0] + inputs[0];
    running_state[1] = running_state[1] + inputs[1];
    let mut next_loc = next_loc_function(running_state[num_locs - 1], num_locs);
    let mut init_state = running_state[num_locs - 1];
    for step in 0..num_steps {
        let next_val = (init_state + running_state[next_loc]) % num_locs;
        // println!("Loc = {:?}, Next val = {:?}", next_loc, next_val);
        running_state[next_loc] = next_val;
        if step < num_steps - 1 {
            next_loc = next_loc_function(next_val, num_locs);
            init_state = next_val;
        }
    }
    running_state[next_loc]
}

fn next_loc_function(val: usize, num_locs: usize) -> usize {
    (3 * val + 1) % num_locs
}

fn usize_to_base_elt(val: usize) -> BaseElement {
    let out: u128 = val.try_into().unwrap();
    BaseElement::from(out)
}

fn usize_to_field<E: FieldElement>(val: usize) -> E {
    let out: u128 = val.try_into().unwrap();
    E::from(out)
}
