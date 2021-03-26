use crate::Example;
use common::Assertions;
use math::field::{BaseElement, FieldElement};

pub fn test_basic_proof_verification(mut e: Box<dyn Example>, size: usize) {
    let assertions = e.prepare(size);
    let proof = e.prove(assertions.clone());
    assert!(e.verify(proof, assertions).is_ok());
}

pub fn test_basic_proof_verification_fail(mut e: Box<dyn Example>, size: usize) {
    let assertions = e.prepare(size);
    let proof = e.prove(assertions.clone());
    let assertions = temper_with_assertions(assertions);
    let verified = e.verify(proof, assertions);
    assert!(verified.is_err());
}

// HELPER FUNCTIONS
// ================================================================================================
fn temper_with_assertions(assertions: Assertions) -> Assertions {
    let mut result = Assertions::new(assertions.trace_width(), assertions.trace_length()).unwrap();
    for (i, assertion) in assertions.into_iter().enumerate() {
        if i == 0 {
            let value = assertion.values()[0] + BaseElement::ONE;
            result
                .add_single(assertion.register(), assertion.first_step(), value)
                .unwrap();
        } else {
            result.add(assertion).unwrap();
        }
    }
    result
}
