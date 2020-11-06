use std::time::Instant;

use log::debug;

use common::errors::VerifierError;
use evaluator::Fib8Evaluator;
use prover::{
    crypto::hash::blake3,
    math::field::{FieldElement, StarkField},
    Assertion, ProofOptions, Prover, StarkProof,
};
use verifier::Verifier;

use super::utils::compute_fib_term;
use crate::Example;

mod evaluator;

// FIBONACCI EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(Fib8Example())
}

pub struct Fib8Example();

impl Example for Fib8Example {
    fn prove(
        &self,
        mut sequence_length: usize,
        mut blowup_factor: usize,
        mut num_queries: usize,
        grinding_factor: u32,
    ) -> (StarkProof, Vec<Assertion>) {
        // apply defaults
        if sequence_length == 0 {
            sequence_length = 1_048_576;
        }
        if blowup_factor == 0 {
            blowup_factor = 16;
        }
        if num_queries == 0 {
            num_queries = 28;
        }

        // compute Fibonacci sequence
        let now = Instant::now();
        let result = compute_fib_term(sequence_length);
        debug!(
            "Computed Fibonacci sequence up to {}th term in {} ms",
            sequence_length,
            now.elapsed().as_millis()
        );

        debug!(
            "Generating proof for computing Fibonacci sequence (8 terms per step) up to {}th term\n\
            ---------------------",
            sequence_length
        );

        // generate execution trace
        let now = Instant::now();
        let trace = build_fib_trace(sequence_length);
        let trace_width = trace.len();
        let trace_length = trace[0].len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace_width,
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // instantiate the prover
        let options = ProofOptions::new(num_queries, blowup_factor, grinding_factor, blake3);
        let prover = Prover::<Fib8Evaluator>::new(options);

        // assert that the trace starts with 7th and 8th terms of Fibonacci sequence (the first
        // 6 terms are not recorded in the trace), and ends with the expected result
        let assertions = vec![
            Assertion::new(0, 0, FieldElement::new(13)),
            Assertion::new(1, 0, FieldElement::new(21)),
            Assertion::new(1, trace_length - 1, result),
        ];
        // generate the proof
        (prover.prove(trace, assertions.clone()).unwrap(), assertions)
    }

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, VerifierError> {
        let verifier = Verifier::<Fib8Evaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// FIBONACCI TRACE BUILDER
// ================================================================================================

pub fn build_fib_trace(length: usize) -> Vec<Vec<FieldElement>> {
    assert!(
        length.is_power_of_two(),
        "sequence length must be a power of 2"
    );

    // initialize the trace with 7th and 8th terms of Fibonacci sequence (skipping the first 6)
    let n0 = FieldElement::ONE;
    let n1 = FieldElement::ONE;
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
