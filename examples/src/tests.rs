use common::Assertions;
use math::field::{BaseElement, FieldElement};

use crate::Example;

pub fn test_basic_proof_verification(
    mut e: Box<dyn Example>,
    this_size: Option<usize>,
    this_blowup_factor: Option<usize>,
    this_num_queries: Option<usize>,
    this_grinding_factor: Option<u32>,
) {
    let size = this_size.unwrap_or(16);
    let blowup_factor = this_blowup_factor.unwrap_or(8);
    let num_queries = this_num_queries.unwrap_or(32);
    let grinding_factor = this_grinding_factor.unwrap_or(0);
    let assertions = e.prepare(size, blowup_factor, num_queries, grinding_factor);
    let proof = e.prove(assertions.clone());
    let verified = e.verify(proof, assertions);
    assert_eq!(true, verified.unwrap());
}

pub fn test_basic_proof_verification_fail(
    mut e: Box<dyn Example>,
    this_size: Option<usize>,
    this_blowup_factor: Option<usize>,
    this_num_queries: Option<usize>,
    this_grinding_factor: Option<u32>,
) {
    let size = this_size.unwrap_or(16);
    let blowup_factor = this_blowup_factor.unwrap_or(8);
    let num_queries = this_num_queries.unwrap_or(32);
    let grinding_factor = this_grinding_factor.unwrap_or(0);
    let assertions = e.prepare(size, blowup_factor, num_queries, grinding_factor);
    let proof = e.prove(assertions.clone());
    let assertions = temper_with_assertions(assertions);
    let verified = e.verify(proof, assertions);
    assert!(verified.is_err());
}

// HELPER FUNCTIONS
// ================================================================================================
fn temper_with_assertions(assertions: Assertions) -> Assertions {
    let mut result = Assertions::new(assertions.trace_width(), assertions.trace_length()).unwrap();

    // copy over point assertions but add 1 to all values
    for assertion in assertions.point_assertions() {
        result
            .add_point(
                assertion.register,
                assertion.step,
                assertion.value + BaseElement::ONE,
            )
            .unwrap();
    }

    // copy over cyclic assertions
    for assertion in assertions.cyclic_assertions() {
        result
            .add_cyclic(
                assertion.register,
                assertion.first_step,
                assertion.values.clone(),
            )
            .unwrap();
    }

    result
}
