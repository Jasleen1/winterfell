use super::Example;
use crate::utils::to_byte_vec;
use log::debug;
use prover::crypto::hash::rescue_s;
use prover::{
    crypto::hash::blake3, Assertion, IoAssertionEvaluator, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::Verifier;

#[allow(clippy::module_inception)]
mod rescue;
mod trace;
use trace::generate_trace;

mod evaluator;
use evaluator::RescueEvaluator;

// CONSTANTS
// ================================================================================================

const CYCLE_LENGTH: usize = 16;
const NUM_HASH_ROUNDS: usize = 14;
const STATE_WIDTH: usize = 4;

// RESCUE HASH CHAIN EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(RescueExample())
}

pub struct RescueExample();

impl Example for RescueExample {
    fn prove(
        &self,
        mut chain_length: usize,
        mut blowup_factor: usize,
        mut num_queries: usize,
    ) -> (StarkProof, Vec<Assertion>) {
        // apply defaults
        if chain_length == 0 {
            chain_length = 1024;
        }
        if blowup_factor == 0 {
            blowup_factor = 32;
        }
        if num_queries == 0 {
            num_queries = 32;
        }

        // initialize a seed for the start of the hash chain
        let seed = [42, 43];

        // compute the sequence of hashes using external implementation of Rescue hash
        let now = Instant::now();
        let expected_result = compute_hash_chain(seed, chain_length);
        debug!(
            "Computed a chain of {} Rescue hashes in {} ms",
            chain_length,
            now.elapsed().as_millis(),
        );

        // generate the execution trace
        debug!(
            "Generating proof for computing a chain of {} Rescue hashes\n\
            ---------------------",
            chain_length
        );
        let now = Instant::now();
        let trace = generate_trace(seed, chain_length);
        let trace_length = trace[0].len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace.len(),
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // make sure execution trace ends up with the same value as the externally
        // computed hash
        let actual_result = [trace[0][trace_length - 1], trace[1][trace_length - 1]];
        assert!(
            expected_result.to_vec() == to_byte_vec(&actual_result),
            "execution trace did not terminate with the expected hash value"
        );

        // instantiate the prover
        let options = ProofOptions::new(num_queries, blowup_factor, 0, blake3);
        let prover = Prover::<RescueEvaluator, IoAssertionEvaluator>::new(options);

        // Assert starting and ending values of the hash chain
        let assertions = vec![
            Assertion::new(0, 0, seed[0]),
            Assertion::new(1, 0, seed[1]),
            Assertion::new(0, trace_length - 1, actual_result[0]),
            Assertion::new(1, trace_length - 1, actual_result[1]),
        ];

        // generate the proof and return it together with the assertions
        (prover.prove(trace, assertions.clone()), assertions)
    }

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, String> {
        let verifier = Verifier::<RescueEvaluator, IoAssertionEvaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn compute_hash_chain(seed: [u128; 2], length: usize) -> [u8; 32] {
    let mut values: Vec<u8> = to_byte_vec(&seed);
    let mut result = [0; 32];

    for _ in 0..length {
        rescue_s(&values, &mut result);
        values.copy_from_slice(&result);
    }

    result
}
