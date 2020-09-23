use super::Example;
use log::debug;
use prover::{
    crypto::hash::blake3,
    math::field::{self, add, mul, sub},
    Assertion, IoAssertionEvaluator, ProofOptions, Prover, StarkProof, TraceInfo,
    TransitionEvaluator,
};
use std::time::Instant;
use verifier::Verifier;

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
        // TODO: clean up
        let verifier = Verifier::<FibEvaluator, IoAssertionEvaluator>::new(proof.options().clone());
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

// FIBONACCI TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct FibEvaluator {
    constraint_degrees: Vec<usize>,
    composition_coefficients: Vec<u128>,
}

impl TransitionEvaluator for FibEvaluator {
    const MAX_CONSTRAINTS: usize = 2;
    const MAX_CONSTRAINT_DEGREE: usize = 1;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(_trace: &TraceInfo, coefficients: &[u128]) -> Self {
        let constraint_degrees = vec![1, 1];
        let composition_coefficients = coefficients[..4].to_vec();

        FibEvaluator {
            constraint_degrees,
            composition_coefficients,
        }
    }

    // TRANSITION CONSTRAINTS
    // --------------------------------------------------------------------------------------------

    fn evaluate(&self, current: &[u128], next: &[u128], _step: usize) -> Vec<u128> {
        self.evaluate_at(current, next, 0)
    }

    fn evaluate_at(&self, current: &[u128], next: &[u128], _x: u128) -> Vec<u128> {
        // expected state width is 2 field elements
        debug_assert_eq!(2, current.len());
        debug_assert_eq!(2, next.len());

        // constraints of Fibonacci sequence which state that:
        // s_{0, i+1} = s_{0, i} + s_{1, i}
        // s_{1, i+1} = s_{0, i} + 2 * s_{1, i}
        vec![
            are_equal(next[0], add(current[0], current[1])),
            are_equal(next[1], add(current[0], mul(2, current[1]))),
        ]
    }

    // BOILERPLATE
    // --------------------------------------------------------------------------------------------
    fn degrees(&self) -> &[usize] {
        &self.constraint_degrees
    }

    fn composition_coefficients(&self) -> &[u128] {
        &self.composition_coefficients
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn are_equal(a: u128, b: u128) -> u128 {
    sub(a, b)
}
