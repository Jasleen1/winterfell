use super::Example;
use common::utils::as_bytes;
use log::debug;
use prover::crypto::hash::rescue_s;
use prover::{
    crypto::hash::blake3, Assertion, IoAssertionEvaluator, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::Verifier;

mod rescue;
mod trace;
use trace::generate_trace;

mod evaluator;
use evaluator::RescueEvaluator;

const CYCLE_LENGTH: usize = 16;
const NUM_HASH_ROUNDS: usize = 14;

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

        // compute the sequence of hashes using external implementation of Rescue hash
        let now = Instant::now();
        let expected_result = compute_hash_chain([42, 43], chain_length);
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
        let trace = generate_trace([42, 43], chain_length);
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
            expected_result == as_bytes(&actual_result),
            "execution trace did not terminate with the expected hash value"
        );

        // instantiate the prover
        let options = ProofOptions::new(num_queries, blowup_factor, 0, blake3);
        let prover = Prover::<RescueEvaluator, IoAssertionEvaluator>::new(options);

        // Generate the proof
        let assertions = vec![
            Assertion::new(0, 0, 42),
            Assertion::new(1, 0, 43),
            Assertion::new(0, trace_length - 1, actual_result[0]),
            Assertion::new(1, trace_length - 1, actual_result[1]),
        ];
        (prover.prove(trace, assertions.clone()), assertions)
    }

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, String> {
        // TODO: clean up
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

fn print_trace(trace: &Vec<Vec<u128>>) {
    let trace_width = trace.len();
    let trace_length = trace[0].len();

    let mut state = vec![0; trace_width];

    for i in 0..trace_length {
        for j in 0..trace_width {
            state[j] = trace[j][i];
        }
        println!("{}\t{:x?}", i, state);
    }
}

fn to_byte_vec(value: &[u128]) -> Vec<u8> {
    as_bytes(value).iter().map(|&b| b).collect()
}
