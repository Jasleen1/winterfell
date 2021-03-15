use crate::{utils::rescue, Example, ExampleOptions};
use log::debug;
use prover::{
    crypto::hash::rescue_s,
    math::field::{BaseElement, FieldElement},
    Assertions, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::{Verifier, VerifierError};

mod trace;
use trace::generate_trace;

mod evaluator;
use evaluator::RescueEvaluator;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

const CYCLE_LENGTH: usize = 16;
const NUM_HASH_ROUNDS: usize = 14;
const STATE_WIDTH: usize = 4;

// RESCUE HASH CHAIN EXAMPLE
// ================================================================================================
pub fn get_example(options: ExampleOptions) -> Box<dyn Example> {
    Box::new(RescueExample::new(options.to_proof_options(28, 32)))
}

pub struct RescueExample {
    options: ProofOptions,
    chain_length: usize,
    seed: [BaseElement; 2],
}

impl RescueExample {
    pub fn new(options: ProofOptions) -> RescueExample {
        RescueExample {
            options,
            chain_length: 0,
            seed: [BaseElement::from(42u8), BaseElement::from(43u8)],
        }
    }
}

// EXAMPLE IMPLEMENTATION
// ================================================================================================

impl Example for RescueExample {
    fn prepare(&mut self, chain_length: usize) -> Assertions {
        assert!(
            chain_length.is_power_of_two(),
            "chain length must a power of 2"
        );
        self.chain_length = chain_length;

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
        let mut assertions = Assertions::new(STATE_WIDTH, last_step + 1).unwrap();
        assertions.add_single(0, 0, self.seed[0]).unwrap();
        assertions.add_single(1, 0, self.seed[1]).unwrap();
        assertions.add_single(0, last_step, result[0]).unwrap();
        assertions.add_single(1, last_step, result[1]).unwrap();
        assertions
    }

    fn prove(&self, assertions: Assertions) -> StarkProof {
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
        let prover = Prover::<RescueEvaluator>::new(self.options.clone());
        prover.prove(trace, assertions).unwrap()
    }

    fn verify(&self, proof: StarkProof, assertions: Assertions) -> Result<(), VerifierError> {
        let verifier = Verifier::<RescueEvaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn compute_hash_chain(seed: [BaseElement; 2], length: usize) -> [u8; 32] {
    let mut values: Vec<u8> = BaseElement::write_into_vec(&seed);
    let mut result = [0; 32];

    for _ in 0..length {
        rescue_s(&values, &mut result);
        values.copy_from_slice(&result);
    }

    result
}
