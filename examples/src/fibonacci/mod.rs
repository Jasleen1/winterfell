use super::Example;
use log::debug;
use prover::{
    crypto::hash::blake3,
    math::field::{self, add, mul},
    Assertion, IoAssertionEvaluator, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::Verifier;

mod evaluator;
use evaluator::FibEvaluator;

// FIBONACCI EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(FibExample())
}

pub struct FibExample();

impl Example for FibExample {
    fn prove(
        &self,
        mut sequence_length: usize,
        mut blowup_factor: usize,
        mut num_queries: usize,
    ) -> (StarkProof, Vec<Assertion>) {
        // apply defaults
        if sequence_length == 0 {
            sequence_length = 1_048_576;
        }
        if blowup_factor == 0 {
            blowup_factor = 8;
        }
        if num_queries == 0 {
            num_queries = 32;
        }

        debug!(
            "Generating proof for computing Fibonacci sequence up to {}th term\n\
            ---------------------",
            sequence_length
        );

        // generate execution trace
        let now = Instant::now();
        let trace = build_fib_trace(sequence_length);

        let trace_width = trace.len();
        let trace_length = trace[0].len();
        let result = trace[1][trace_length - 1];
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace_width,
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // instantiate the prover
        let options = ProofOptions::new(num_queries, blowup_factor, 0, blake3);
        let prover = Prover::<FibEvaluator, IoAssertionEvaluator>::new(options);

        // Generate the proof
        let assertions = vec![
            Assertion::new(0, 0, 1),
            Assertion::new(1, 0, 1),
            Assertion::new(1, trace_length - 1, result),
        ];
        (prover.prove(trace, assertions.clone()), assertions)
    }

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, String> {
        let verifier = Verifier::<FibEvaluator, IoAssertionEvaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// FIBONACCI TRACE BUILDER
// ================================================================================================

pub fn build_fib_trace(length: usize) -> Vec<Vec<u128>> {
    assert!(
        length.is_power_of_two(),
        "sequence length must be a power of 2"
    );

    let mut reg1 = vec![field::ONE];
    let mut reg2 = vec![field::ONE];

    for i in 0..(length / 2 - 1) {
        reg1.push(add(reg1[i], reg2[i]));
        reg2.push(add(reg1[i], mul(2, reg2[i])));
    }

    vec![reg1, reg2]
}
