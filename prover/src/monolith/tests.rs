use super::Prover;
use crate::tests::{build_fib_trace, FibEvaluator};
use common::stark::{Assertion, ProofOptions};
use crypto::hash::blake3;
use math::field::FieldElement;

#[test]
fn generate_proof() {
    let trace_length = 8;
    let options = ProofOptions::new(20, 4, 0, blake3);
    let trace = build_fib_trace(trace_length * 2);
    let result = trace[1][trace_length - 1];
    let assertions = vec![
        Assertion::new(0, 0, FieldElement::from(1u8)),
        Assertion::new(1, 0, FieldElement::from(1u8)),
        Assertion::new(1, trace_length - 1, result),
    ];

    let prover = Prover::<FibEvaluator>::new(options);
    let _proof = prover.prove(trace, assertions);
    // TODO: verify that the proof is valid
}
