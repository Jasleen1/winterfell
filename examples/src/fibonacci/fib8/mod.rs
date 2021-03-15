use super::utils::compute_fib_term;
use crate::{Example, ExampleOptions};
use log::debug;
use prover::{
    math::field::{BaseElement, FieldElement},
    Assertions, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::{Verifier, VerifierError};

mod evaluator;
use evaluator::Fib8Evaluator;

#[cfg(test)]
mod tests;

// FIBONACCI EXAMPLE
// ================================================================================================
const TRACE_WIDTH: usize = 8;

pub fn get_example(options: ExampleOptions) -> Box<dyn Example> {
    Box::new(Fib8Example::new(options.to_proof_options(28, 16)))
}

pub struct Fib8Example {
    options: ProofOptions,
    sequence_length: usize,
}

impl Fib8Example {
    pub fn new(options: ProofOptions) -> Fib8Example {
        Fib8Example {
            options,
            sequence_length: 0,
        }
    }
}

// EXAMPLE IMPLEMENTATION
// ================================================================================================

impl Example for Fib8Example {
    fn prepare(&mut self, sequence_length: usize) -> Assertions {
        assert!(
            sequence_length.is_power_of_two(),
            "sequence length must be a power of 2"
        );
        self.sequence_length = sequence_length;
        let trace_length = sequence_length / 8;

        // compute Fibonacci sequence
        let now = Instant::now();
        let result = compute_fib_term(sequence_length);
        debug!(
            "Computed Fibonacci sequence up to {}th term in {} ms",
            sequence_length,
            now.elapsed().as_millis()
        );

        // assert that the trace starts with 7th and 8th terms of Fibonacci sequence (the first
        // 6 terms are not recorded in the trace), and ends with the expected result
        let mut assertions = Assertions::new(TRACE_WIDTH, trace_length).unwrap();
        assertions.add_single(0, 0, BaseElement::new(13)).unwrap();
        assertions.add_single(1, 0, BaseElement::new(21)).unwrap();
        assertions.add_single(1, trace_length - 1, result).unwrap();
        assertions
    }

    fn prove(&self, assertions: Assertions) -> StarkProof {
        debug!(
            "Generating proof for computing Fibonacci sequence (8 terms per step) up to {}th term\n\
            ---------------------",
            self.sequence_length
        );

        // generate execution trace
        let now = Instant::now();
        let trace = build_fib_trace(self.sequence_length);
        let trace_width = trace.len();
        let trace_length = trace[0].len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace_width,
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // generate the proof
        let prover = Prover::<Fib8Evaluator>::new(self.options.clone());
        prover.prove(trace, assertions).unwrap()
    }

    fn verify(&self, proof: StarkProof, assertions: Assertions) -> Result<(), VerifierError> {
        let verifier = Verifier::<Fib8Evaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// FIBONACCI TRACE BUILDER
// ================================================================================================

pub fn build_fib_trace(length: usize) -> Vec<Vec<BaseElement>> {
    assert!(
        length.is_power_of_two(),
        "sequence length must be a power of 2"
    );

    // initialize the trace with 7th and 8th terms of Fibonacci sequence (skipping the first 6)
    let n0 = BaseElement::ONE;
    let n1 = BaseElement::ONE;
    let n2 = n0 + n1;
    let n3 = n1 + n2;
    let n4 = n2 + n3;
    let n5 = n3 + n4;
    let n6 = n4 + n5;
    let n7 = n5 + n6;

    let mut reg0 = vec![n6];
    let mut reg1 = vec![n7];

    for i in 0..(length / 8 - 1) {
        let n0 = reg0[i] + reg1[i];
        let n1 = reg1[i] + n0;
        let n2 = n0 + n1;
        let n3 = n1 + n2;
        let n4 = n2 + n3;
        let n5 = n3 + n4;
        let n6 = n4 + n5;
        let n7 = n5 + n6;

        reg0.push(n6);
        reg1.push(n7);
    }

    vec![reg0, reg1]
}
