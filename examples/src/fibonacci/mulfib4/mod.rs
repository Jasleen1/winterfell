use std::time::Instant;

use log::debug;

use common::errors::VerifierError;
use evaluator::MulFib4Evaluator;
use prover::{
    crypto::hash::blake3, math::field::FieldElement, Assertion, ProofOptions, Prover, StarkProof,
};
use verifier::Verifier;

use super::utils::compute_mulfib_term;
use crate::Example;

mod evaluator;

// FIBONACCI EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(MulFib4Example())
}

pub struct MulFib4Example();

impl Example for MulFib4Example {
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
        let result = compute_mulfib_term(sequence_length);
        debug!(
            "Computed multiplicative Fibonacci sequence up to {}th term in {} ms",
            sequence_length,
            now.elapsed().as_millis()
        );

        debug!(
            "Generating proof for computing multiplicative Fibonacci sequence (4 terms per step) up to {}th term\n\
            ---------------------",
            sequence_length
        );

        // generate execution trace
        let now = Instant::now();
        let trace = build_mulfib_trace(sequence_length);
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
        let prover = Prover::<MulFib4Evaluator>::new(options);

        // Generate the proof
        let assertions = vec![
            Assertion::new(0, 0, FieldElement::new(1)),
            Assertion::new(1, 0, FieldElement::new(2)),
            Assertion::new(2, trace_length - 1, result),
        ];
        (prover.prove(trace, assertions.clone()).unwrap(), assertions)
    }

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, VerifierError> {
        let verifier = Verifier::<MulFib4Evaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// FIBONACCI TRACE BUILDER
// ================================================================================================

pub fn build_mulfib_trace(length: usize) -> Vec<Vec<FieldElement>> {
    assert!(
        length.is_power_of_two(),
        "sequence length must be a power of 2"
    );

    let mut reg0 = vec![FieldElement::new(1)];
    let mut reg1 = vec![FieldElement::new(2)];
    let mut reg2 = vec![reg0[0] * reg1[0]];
    let mut reg3 = vec![reg1[0] * reg2[0]];

    for i in 0..(length / 4 - 1) {
        reg0.push(reg2[i] * reg3[i]);
        reg1.push(reg3[i] * reg0[i + 1]);
        reg2.push(reg0[i + 1] * reg1[i + 1]);
        reg3.push(reg1[i + 1] * reg2[i + 1]);
    }

    vec![reg0, reg1, reg2, reg3]
}
