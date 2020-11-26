use common::errors::VerifierError;
use evaluator::MulFib8Evaluator;
use log::debug;
use prover::{
    crypto::hash::blake3, math::field::BaseElement, Assertion, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::Verifier;

use super::utils::compute_mulfib_term;
use crate::Example;

mod evaluator;

// FIBONACCI EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(MulFib8Example())
}

const NUM_REGISTERS: usize = 8;

pub struct MulFib8Example();

impl Example for MulFib8Example {
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
            "Generating proof for computing multiplicative Fibonacci sequence (8 terms per step) up to {}th term\n\
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
        let prover = Prover::<MulFib8Evaluator>::new(options);

        // Generate the proof
        let assertions = vec![
            Assertion::new(0, 0, BaseElement::new(1)),
            Assertion::new(1, 0, BaseElement::new(2)),
            Assertion::new(6, trace_length - 1, result),
        ];
        (prover.prove(trace, assertions.clone()).unwrap(), assertions)
    }

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, VerifierError> {
        let verifier = Verifier::<MulFib8Evaluator>::new();
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
    let mut reg2 = vec![reg0[0] * reg1[0]];
    let mut reg3 = vec![reg1[0] * reg2[0]];
    let mut reg4 = vec![reg2[0] * reg3[0]];
    let mut reg5 = vec![reg3[0] * reg4[0]];
    let mut reg6 = vec![reg4[0] * reg5[0]];
    let mut reg7 = vec![reg5[0] * reg6[0]];

    for i in 0..(length / 8 - 1) {
        reg0.push(reg6[i] * reg7[i]);
        reg1.push(reg7[i] * reg0[i + 1]);
        reg2.push(reg0[i + 1] * reg1[i + 1]);
        reg3.push(reg1[i + 1] * reg2[i + 1]);
        reg4.push(reg2[i + 1] * reg3[i + 1]);
        reg5.push(reg3[i + 1] * reg4[i + 1]);
        reg6.push(reg4[i + 1] * reg5[i + 1]);
        reg7.push(reg5[i + 1] * reg6[i + 1]);
    }

    vec![reg0, reg1, reg2, reg3, reg4, reg5, reg6, reg7]
}
