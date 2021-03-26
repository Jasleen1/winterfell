use super::Prover;
use crate::tests::{build_fib_trace, FibEvaluator};
use common::{Assertions, FieldExtension, ProofOptions};
use crypto::hash::blake3;
use math::field::BaseElement;

#[test]
fn generate_proof() {
    let trace_length = 8;
    let options = ProofOptions::new(20, 4, 0, blake3, FieldExtension::None);
    let trace = build_fib_trace(trace_length * 2);
    let result = trace.get(1, trace_length - 1);
    let mut assertions = Assertions::new(trace.len(), trace_length).unwrap();
    assertions.add_single(0, 0, BaseElement::from(1u8)).unwrap();
    assertions.add_single(1, 0, BaseElement::from(1u8)).unwrap();
    assertions.add_single(1, trace_length - 1, result).unwrap();

    let prover = Prover::<FibEvaluator>::new(options);
    let _proof = prover.prove(trace, assertions);
    // TODO: verify that the proof is valid
}
