use common::errors::VerifierError;
use log::debug;
use prover::{
    math::field::{BaseElement, FieldElement},
    Assertion, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::Verifier;

use super::utils::{build_proof_options, compute_fib_term};
use crate::Example;

mod evaluator;
use evaluator::FibEvaluator;

// FIBONACCI EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(FibExample {
        options: None,
        sequence_length: 0,
    })
}

pub struct FibExample {
    options: Option<ProofOptions>,
    sequence_length: usize,
}

impl Example for FibExample {
    fn prepare(
        &mut self,
        mut sequence_length: usize,
        blowup_factor: usize,
        num_queries: usize,
        grinding_factor: u32,
    ) -> Vec<Assertion> {
        if sequence_length == 0 {
            sequence_length = 1_048_576
        }
        self.sequence_length = sequence_length;
        self.options = build_proof_options(blowup_factor, num_queries, grinding_factor);
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
        vec![
            Assertion::new(0, 0, BaseElement::ONE),
            Assertion::new(1, 0, BaseElement::ONE),
            Assertion::new(1, trace_length - 1, result),
        ]
    }

    fn prove(&self, assertions: Vec<Assertion>) -> StarkProof {
        debug!(
            "Generating proof for computing Fibonacci sequence (2 terms per step) up to {}th term\n\
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
        let prover = Prover::<FibEvaluator>::new(self.options.clone().unwrap());
        prover.prove(trace, assertions).unwrap()
    }

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, VerifierError> {
        let verifier = Verifier::<FibEvaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// FIBONACCI TRACE BUILDER
// ================================================================================================

fn build_fib_trace(length: usize) -> Vec<Vec<BaseElement>> {
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

    vec![reg0, reg1]
}
