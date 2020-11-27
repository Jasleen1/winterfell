use common::errors::VerifierError;
use log::debug;
use prover::{
    crypto::hash::{blake3, rescue_s},
    math::field::{BaseElement, FieldElement},
    Assertion, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::Verifier;

use super::Example;
use crate::utils::rescue;

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
    Box::new(RescueExample {
        options: None,
        chain_length: 0,
        seed: [BaseElement::from(42u8), BaseElement::from(43u8)],
    })
}

pub struct RescueExample {
    options: Option<ProofOptions>,
    chain_length: usize,
    seed: [BaseElement; 2],
}

impl Example for RescueExample {
    fn prepare(
        &mut self,
        chain_length: usize,
        blowup_factor: usize,
        num_queries: usize,
        grinding_factor: u32,
    ) -> Vec<Assertion> {
        self.options = build_proof_options(blowup_factor, num_queries, grinding_factor);
        self.chain_length = if chain_length == 0 {
            1024
        } else {
            chain_length
        };

        // compute the sequence of hashes using external implementation of Rescue hash
        let now = Instant::now();
        let result = compute_hash_chain(self.seed, self.chain_length);
        debug!(
            "Computed a chain of {} Rescue hashes in {} ms",
            self.chain_length,
            now.elapsed().as_millis(),
        );

        // Assert starting and ending values of the hash chain
        let last_step = (self.chain_length * 16) - 1;
        let result = BaseElement::read_to_vec(&result).unwrap();
        vec![
            Assertion::new(0, 0, self.seed[0]),
            Assertion::new(1, 0, self.seed[1]),
            Assertion::new(0, last_step, result[0]),
            Assertion::new(1, last_step, result[1]),
        ]
    }

    fn prove(&self, assertions: Vec<Assertion>) -> StarkProof {
        // generate the execution trace
        debug!(
            "Generating proof for computing a chain of {} Rescue hashes\n\
            ---------------------",
            self.chain_length
        );
        let now = Instant::now();
        let trace = generate_trace(self.seed, self.chain_length);
        let trace_length = trace[0].len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace.len(),
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // generate the proof
        let prover = Prover::<RescueEvaluator>::new(self.options.clone().unwrap());
        prover.prove(trace, assertions).unwrap()
    }

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, VerifierError> {
        let verifier = Verifier::<RescueEvaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_proof_options(
    mut blowup_factor: usize,
    mut num_queries: usize,
    grinding_factor: u32,
) -> Option<ProofOptions> {
    if blowup_factor == 0 {
        blowup_factor = 32;
    }
    if num_queries == 0 {
        num_queries = 28;
    }
    let options = ProofOptions::new(num_queries, blowup_factor, grinding_factor, blake3);
    Some(options)
}

fn compute_hash_chain(seed: [BaseElement; 2], length: usize) -> [u8; 32] {
    let mut values: Vec<u8> = BaseElement::write_into_vec(&seed);
    let mut result = [0; 32];

    for _ in 0..length {
        rescue_s(&values, &mut result);
        values.copy_from_slice(&result);
    }

    result
}
