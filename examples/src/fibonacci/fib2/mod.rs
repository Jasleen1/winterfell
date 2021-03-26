use super::utils::compute_fib_term;
use crate::{Example, ExampleOptions};
use log::debug;
use prover::{
    math::field::{BaseElement, FieldElement},
    Assertions, ExecutionTrace, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::{Verifier, VerifierError};

mod evaluator;
use evaluator::FibEvaluator;

#[cfg(test)]
mod tests;

// FIBONACCI EXAMPLE
// ================================================================================================
const TRACE_WIDTH: usize = 2;

pub fn get_example(options: ExampleOptions) -> Box<dyn Example> {
    Box::new(FibExample::new(options.to_proof_options(28, 16)))
}

pub struct FibExample {
    options: ProofOptions,
    sequence_length: usize,
}

impl FibExample {
    pub fn new(options: ProofOptions) -> FibExample {
        FibExample {
            options,
            sequence_length: 0,
        }
    }
}

// EXAMPLE IMPLEMENTATION
// ================================================================================================

impl Example for FibExample {
    fn prepare(&mut self, sequence_length: usize) -> Assertions {
        assert!(
            sequence_length.is_power_of_two(),
            "sequence length must be a power of 2"
        );
        self.sequence_length = sequence_length;
        let trace_length = sequence_length / 2;

        // compute Fibonacci sequence
        let now = Instant::now();
        let result = compute_fib_term(sequence_length);
        debug!(
            "Computed Fibonacci sequence up to {}th term in {} ms",
            sequence_length,
            now.elapsed().as_millis()
        );

        // a valid Fibonacci sequence should start with two ones and terminate with
        // the same result as computed above
        let mut assertions = Assertions::new(TRACE_WIDTH, trace_length).unwrap();
        assertions.add_single(0, 0, BaseElement::ONE).unwrap();
        assertions.add_single(1, 0, BaseElement::ONE).unwrap();
        assertions.add_single(1, trace_length - 1, result).unwrap();
        assertions
    }

    fn prove(&self, assertions: Assertions) -> StarkProof {
        debug!(
            "Generating proof for computing Fibonacci sequence (2 terms per step) up to {}th term\n\
            ---------------------",
            self.sequence_length
        );

        // generate execution trace
        let now = Instant::now();
        let trace = build_fib_trace(self.sequence_length);

        let trace_width = trace.width();
        let trace_length = trace.len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace_width,
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // generate the proof
        let prover = Prover::<FibEvaluator>::new(self.options.clone());
        prover.prove(trace, assertions).unwrap()
    }

    fn verify(&self, proof: StarkProof, assertions: Assertions) -> Result<(), VerifierError> {
        let verifier = Verifier::<FibEvaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// FIBONACCI TRACE BUILDER
// ================================================================================================
fn build_fib_trace(length: usize) -> ExecutionTrace {
    assert!(
        length.is_power_of_two(),
        "sequence length must be a power of 2"
    );

    let mut reg0 = vec![BaseElement::ONE];
    let mut reg1 = vec![BaseElement::ONE];

    for i in 0..(length / 2 - 1) {
        reg0.push(reg0[i] + reg1[i]);
        reg1.push(reg1[i] + reg0[i + 1]);
    }

    ExecutionTrace::init(vec![reg0, reg1])
}
