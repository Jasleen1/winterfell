use super::utils::compute_mulfib_term;
use crate::{Example, ExampleOptions};
use log::debug;
use prover::{
    math::{field::BaseElement, utils::log2},
    Assertions, ExecutionTrace, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::{Verifier, VerifierError};

mod evaluator;
use evaluator::MulFib8Evaluator;

#[cfg(test)]
mod tests;

// FIBONACCI EXAMPLE
// ================================================================================================
const NUM_REGISTERS: usize = 8;

pub fn get_example(options: ExampleOptions) -> Box<dyn Example> {
    Box::new(MulFib8Example::new(options.to_proof_options(28, 16)))
}

pub struct MulFib8Example {
    options: ProofOptions,
    sequence_length: usize,
}

impl MulFib8Example {
    pub fn new(options: ProofOptions) -> MulFib8Example {
        MulFib8Example {
            options,
            sequence_length: 0,
        }
    }
}

// EXAMPLE IMPLEMENTATION
// ================================================================================================

impl Example for MulFib8Example {
    fn prepare(&mut self, sequence_length: usize) -> Assertions {
        assert!(
            sequence_length.is_power_of_two(),
            "sequence length must be a power of 2"
        );
        self.sequence_length = sequence_length;
        let trace_length = sequence_length / 8;

        // compute Fibonacci sequence
        let now = Instant::now();
        let result = compute_mulfib_term(sequence_length);
        debug!(
            "Computed multiplicative Fibonacci sequence up to {}th term in {} ms",
            sequence_length,
            now.elapsed().as_millis()
        );

        let mut assertions = Assertions::new(NUM_REGISTERS, trace_length).unwrap();
        assertions.add_single(0, 0, BaseElement::new(1)).unwrap();
        assertions.add_single(1, 0, BaseElement::new(2)).unwrap();
        assertions.add_single(6, trace_length - 1, result).unwrap();
        assertions
    }

    fn prove(&self, assertions: Assertions) -> StarkProof {
        let sequence_length = self.sequence_length;
        debug!(
            "Generating proof for computing multiplicative Fibonacci sequence (8 terms per step) up to {}th term\n\
            ---------------------",
            sequence_length
        );

        // generate execution trace
        let now = Instant::now();
        let trace = build_mulfib_trace(sequence_length);
        let trace_width = trace.width();
        let trace_length = trace.len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace_width,
            log2(trace_length),
            now.elapsed().as_millis()
        );

        // generate the proof
        let prover = Prover::<MulFib8Evaluator>::new(self.options.clone());
        prover.prove(trace, assertions).unwrap()
    }

    fn verify(&self, proof: StarkProof, assertions: Assertions) -> Result<(), VerifierError> {
        let verifier = Verifier::<MulFib8Evaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// FIBONACCI TRACE BUILDER
// ================================================================================================

pub fn build_mulfib_trace(length: usize) -> ExecutionTrace {
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

    ExecutionTrace::init(vec![reg0, reg1, reg2, reg3, reg4, reg5, reg6, reg7])
}
