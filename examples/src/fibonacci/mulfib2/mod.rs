use common::errors::VerifierError;
use log::debug;
use prover::{math::field::BaseElement, Assertion, ProofOptions, Prover, StarkProof};
use std::time::Instant;
use verifier::Verifier;

use super::utils::{build_proof_options, compute_mulfib_term};
use crate::Example;

mod evaluator;
use evaluator::MulFib2Evaluator;

#[cfg(test)]
mod tests;

// FIBONACCI EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(MulFib2Example {
        options: None,
        sequence_length: 0,
    })
}

const NUM_REGISTERS: usize = 2;

pub struct MulFib2Example {
    options: Option<ProofOptions>,
    sequence_length: usize,
}

impl Example for MulFib2Example {
    fn prepare(
        &mut self,
        sequence_length: usize,
        blowup_factor: usize,
        num_queries: usize,
        grinding_factor: u32,
    ) -> Vec<Assertion> {
        self.sequence_length = if sequence_length == 0 {
            1_048_576
        } else {
            sequence_length
        };
        self.options = build_proof_options(blowup_factor, num_queries, grinding_factor);
        let trace_length = sequence_length / 2;

        // compute Fibonacci sequence
        let now = Instant::now();
        let result = compute_mulfib_term(sequence_length);
        debug!(
            "Computed multiplicative Fibonacci sequence up to {}th term in {} ms",
            sequence_length,
            now.elapsed().as_millis()
        );

        vec![
            Assertion::new(0, 0, BaseElement::new(1)),
            Assertion::new(1, 0, BaseElement::new(2)),
            Assertion::new(0, trace_length - 1, result),
        ]
    }

    fn prove(&self, assertions: Vec<Assertion>) -> StarkProof {
        let sequence_length = self.sequence_length;
        debug!(
            "Generating proof for computing multiplicative Fibonacci sequence (2 terms per step) up to {}th term\n\
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

        // generate the proof
        let prover = Prover::<MulFib2Evaluator>::new(self.options.clone().unwrap());
        prover.prove(trace, assertions).unwrap()
    }

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, VerifierError> {
        let verifier = Verifier::<MulFib2Evaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// FIBONACCI TRACE BUILDER
// ================================================================================================

pub fn build_mulfib_trace(length: usize) -> Vec<Vec<BaseElement>> {
    assert!(
        length.is_power_of_two(),
        "sequence length must be a power of 2"
    );

    let mut reg0 = vec![BaseElement::new(1)];
    let mut reg1 = vec![BaseElement::new(2)];

    for i in 0..(length / 2 - 1) {
        reg0.push(reg0[i] * reg1[i]);
        reg1.push(reg1[i] * reg0[i + 1]);
    }

    vec![reg0, reg1]
}
