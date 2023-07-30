// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::{utils, Blake3_192, Blake3_256, Example, ExampleOptions, HashFunction, Sha3_256};
use core::marker::PhantomData;
use log::debug;
use rand_utils::{rand_array, rand_value};
use std::{num, time::Instant};
use winterfell::{
    crypto::{
        hashers::{Rp64_256, RpJive64_256},
        DefaultRandomCoin, ElementHasher,
    },
    math::{fields::f128::BaseElement, ExtensionOf, FieldElement},
    ProofOptions, Prover, StarkProof, Trace, VerifierError,
};

use super::rescue::rescue::{self, STATE_WIDTH};

mod air;
use air::{PublicInputs, RamConstraintsAir};

mod prover;
use prover::RamConstraintProver;

#[cfg(test)]
mod tests;

// RESCUE SPLIT HASH CHAIN EXAMPLE
// ================================================================================================

pub fn get_example(
    options: &ExampleOptions,
    num_ram_steps: usize,
    num_locs: usize,
) -> Result<Box<dyn Example>, String> {
    let (options, hash_fn) = options.to_proof_options(42, 64);

    assert!(num_locs.is_power_of_two());
    assert!(num_ram_steps.is_power_of_two());
    match hash_fn {
        HashFunction::Blake3_192 => Ok(Box::new(RamConstraintsExample::<Blake3_192>::new(
            num_locs,
            num_ram_steps,
            options,
        ))),
        HashFunction::Blake3_256 => Ok(Box::new(RamConstraintsExample::<Blake3_256>::new(
            num_locs,
            num_ram_steps,
            options,
        ))),
        HashFunction::Sha3_256 => Ok(Box::new(RamConstraintsExample::<Sha3_256>::new(
            num_locs,
            num_ram_steps,
            options,
        ))),
        _ => Err("The specified hash function cannot be used with this example.".to_string()),
    }
}

pub struct RamConstraintsExample<H: ElementHasher> {
    options: ProofOptions,
    num_locs: usize,
    num_ram_steps: usize,
    valid_ram: Vec<[u64; 4]>,
    _hasher: PhantomData<H>,
}

impl<H: ElementHasher> RamConstraintsExample<H> {
    pub fn new(num_locs: usize, num_ram_steps: usize, options: ProofOptions) -> Self {
        assert!(
            num_locs.is_power_of_two(),
            "number of locations must a power of 2"
        );
        assert!(
            num_ram_steps.is_power_of_two(),
            "number of RAM steps must a power of 2"
        );
        assert!(num_locs <= num_ram_steps, "Want more steps than locations");

        // compute a valid ram
        let now = Instant::now();
        let valid_ram = compute_valid_ram(num_locs, num_ram_steps);

        debug!(
            "Computed a RAM example for {} locations and {} steps in {} ms",
            num_locs,
            num_ram_steps,
            now.elapsed().as_millis(),
        );

        RamConstraintsExample {
            options,
            num_locs,
            num_ram_steps,
            valid_ram,
            _hasher: PhantomData,
        }
    }
}

// EXAMPLE IMPLEMENTATION
// ================================================================================================

impl<H: ElementHasher> Example for RamConstraintsExample<H>
where
    H: ElementHasher<BaseField = BaseElement>,
{
    fn prove(&self) -> StarkProof {
        // generate the execution trace
        debug!(
            "Generating proof for correct RAM with {} locs and {} steps\n\
            ---------------------",
            self.num_locs, self.num_ram_steps,
        );

        // create a prover
        let prover =
            RamConstraintProver::<H>::new(self.options.clone(), self.num_locs, self.num_ram_steps);

        // generate the execution trace
        let now = Instant::now();
        let trace = prover.build_trace(&self.valid_ram);
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
            num_locs: self.num_locs.try_into().unwrap(),
            num_ram_steps: self.num_ram_steps.try_into().unwrap(),
        };
        winterfell::verify::<RamConstraintsAir, H, DefaultRandomCoin<H>>(proof, pub_inputs)
    }

    fn verify_with_wrong_inputs(&self, proof: StarkProof) -> Result<(), VerifierError> {
        let pub_inputs = PublicInputs {
            num_locs: self.num_locs.try_into().unwrap(),
            num_ram_steps: self.num_ram_steps.try_into().unwrap(),
        };
        winterfell::verify::<RamConstraintsAir, H, DefaultRandomCoin<H>>(proof, pub_inputs)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn compute_valid_ram(num_locs: usize, num_ram_steps: usize) -> Vec<[u64; 4]> {
    let mut access_vec = Vec::<[u64; 4]>::new();
    let locs_64: u64 = num_locs.try_into().unwrap();
    let mut running_state: Vec<u64> = (0u64..locs_64).collect();
    for step in 0u64..locs_64 {
        let step_usize: usize = step.try_into().unwrap();
        access_vec.push([step, 1, step, running_state[step_usize]]);
    }
    let num_remaining_steps = num_ram_steps - num_locs;
    let remaining_steps_u64: u64 = num_remaining_steps.try_into().unwrap();
    for step in 0u64..remaining_steps_u64 {
        let op = step % 2;
        let loc: u64 = (access_vec[access_vec.len() - 1][3] * 10) % locs_64;
        let loc_usize: usize = loc.try_into().unwrap();
        let val = if op == 0 {
            running_state[loc_usize]
        } else {
            rand_value::<u64>() % locs_64
        };
        access_vec.push([step + locs_64, op, loc, val]);
        running_state[loc_usize] = val;
    }
    sort_accesses_in_place(&mut access_vec);
    access_vec
}

fn sort_accesses_in_place(accesses: &mut Vec<[u64; 4]>) {
    // Rust's sort_by* functions are "stable" i.e. do not reorder
    // equal values. Since we expect the input to be already ordered by
    // the 0th entry of each tuple, this will give us an array of 4-tuples,
    // sorted by the 2nd entry and 0th entry used for tie-breaks.
    accesses.sort_by_key(|a| a[2]);
}
