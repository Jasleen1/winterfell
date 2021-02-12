use super::*;
use crate::{ComputationContext, FieldExtension, ProofOptions};
use crypto::{hash::blake3, RandomElementGenerator};
use math::field::{BaseElement, FieldElement};

// ASSERTION TESTS
// ================================================================================================

#[test]
fn create_assertions_collection() {
    assert_eq!(
        Err(AssertionError::TraceWidthTooShort),
        Assertions::new(0, 4)
    );
    assert_eq!(
        Err(AssertionError::TraceLengthNotPowerOfTwo(3)),
        Assertions::new(1, 3)
    );
    assert!(Assertions::new(1, 1).is_ok());
}

#[test]
fn add_point_assertion() {
    let mut assertions = Assertions::new(2, 4).unwrap();

    assert_eq!(
        Err(AssertionError::InvalidAssertionRegisterIndex(2)),
        assertions.add_point(2, 0, BaseElement::ONE)
    );
    assert_eq!(
        Err(AssertionError::InvalidAssertionStep(4)),
        assertions.add_point(1, 4, BaseElement::ONE)
    );

    assert!(assertions.add_point(0, 1, BaseElement::ONE).is_ok());
    assert!(assertions.add_point(1, 1, BaseElement::ONE).is_ok());
    assert_eq!(
        Err(AssertionError::DuplicateAssertion(1, 1)),
        assertions.add_point(1, 1, BaseElement::ONE)
    );
}

#[test]
fn add_cyclic_assertion() {
    let mut assertions = Assertions::new(2, 16).unwrap();

    let values = vec![
        BaseElement::new(1),
        BaseElement::new(2),
        BaseElement::new(3),
        BaseElement::new(4),
    ];

    assert_eq!(
        Err(AssertionError::NumberOfValuesNotPowerOfTwo(3)),
        assertions.add_cyclic(0, 0, values[..3].to_vec())
    );

    assert_eq!(
        Err(AssertionError::InvalidAssertionRegisterIndex(2)),
        assertions.add_cyclic(2, 0, values.clone())
    );

    assert_eq!(
        Err(AssertionError::TooManyCyclicAssertionValues(16)),
        assertions.add_cyclic(0, 0, vec![BaseElement::ONE; 16])
    );

    assert_eq!(
        Err(AssertionError::InvalidFirstCycleStart(4, 4)),
        assertions.add_cyclic(0, 4, values.clone())
    );

    // assertions for steps: 0, 4, 8, 12
    assert!(assertions.add_cyclic(0, 0, values.clone()).is_ok());

    // starts on the same steps, has the same number of values
    assert_eq!(
        Err(AssertionError::OverlappingCyclicAssertion(0, 4)),
        assertions.add_cyclic(0, 0, values.clone()) // 0, 4, 8, 12
    );

    // starts on the same step, has different number of values
    assert_eq!(
        Err(AssertionError::OverlappingCyclicAssertion(0, 8)),
        assertions.add_cyclic(0, 0, values[0..2].to_vec()) // 0, 8
    );

    // assertions for steps: 1, 5, 9, 13
    assert!(assertions.add_cyclic(0, 1, values.clone()).is_ok());

    // starts on different step, but existing catches up
    assert_eq!(
        Err(AssertionError::OverlappingCyclicAssertion(5, 8)),
        assertions.add_cyclic(0, 5, values[0..2].to_vec()) // 5, 13
    );
}

#[test]
fn add_overlapping_assertion() {
    let mut assertions = Assertions::new(2, 16).unwrap();

    assert!(assertions.add_point(0, 0, BaseElement::ONE).is_ok());
    assert!(assertions.add_point(0, 9, BaseElement::ONE).is_ok());

    let values = vec![
        BaseElement::new(1),
        BaseElement::new(2),
        BaseElement::new(3),
        BaseElement::new(4),
    ];

    // covers (0, 0)
    assert_eq!(
        Err(AssertionError::CoveringCyclicAssertion(0, 4)),
        assertions.add_cyclic(0, 0, values.clone())
    );

    // covers (0, 9)
    assert_eq!(
        Err(AssertionError::CoveringCyclicAssertion(1, 4)),
        assertions.add_cyclic(0, 1, values.clone())
    );

    // assertions for steps 2, 6, 10, 14
    assert!(assertions.add_cyclic(0, 2, values.clone()).is_ok());

    assert_eq!(
        Err(AssertionError::AssertionCoveredByCyclicAssertion(0, 2)),
        assertions.add_point(0, 2, BaseElement::ONE)
    );

    assert_eq!(
        Err(AssertionError::AssertionCoveredByCyclicAssertion(0, 10)),
        assertions.add_point(0, 10, BaseElement::ONE)
    );

    assert!(assertions.add_point(0, 11, BaseElement::ONE).is_ok());
}

#[test]
fn assertions_for_each() {
    let mut assertions = Assertions::new(2, 16).unwrap();

    let values = vec![
        BaseElement::new(1),
        BaseElement::new(2),
        BaseElement::new(3),
        BaseElement::new(4),
    ];

    assert!(assertions.add_point(0, 9, BaseElement::new(5)).is_ok());
    assert!(assertions.add_point(0, 0, BaseElement::new(3)).is_ok());
    // assertions for steps 2, 6, 10, 14
    assert!(assertions.add_cyclic(0, 2, values.clone()).is_ok());
    // assertions for steps 0, 8
    assert!(assertions.add_cyclic(1, 0, values[..2].to_vec()).is_ok());

    let expected = vec![
        (0, 0, BaseElement::new(3)),
        (0, 9, BaseElement::new(5)),
        (0, 2, BaseElement::new(1)),
        (0, 6, BaseElement::new(2)),
        (0, 10, BaseElement::new(3)),
        (0, 14, BaseElement::new(4)),
        (1, 0, BaseElement::new(1)),
        (1, 8, BaseElement::new(2)),
    ];

    let mut actual = Vec::new();
    assertions.for_each(|reg, step, value| {
        actual.push((reg, step, value));
    });

    assert_eq!(expected, actual);
}

// CONSTRAINT TESTS
// ================================================================================================
#[test]
fn build_assertion_constraints_one_cyclic_assertion() {
    // set up computation context
    let trace_length = 16;
    let context = build_context(trace_length);
    let coeff_prng = RandomElementGenerator::new([1; 32], 0, blake3);
    let domain = BaseElement::get_power_series(context.generators().trace_domain, trace_length);

    // initialize assertions collection
    let mut assertions = super::Assertions::new(1, trace_length).unwrap();

    // add an assertion specifying that the following should hold for register 0:
    // assert(step = 0) = 1,
    // assert(step = 4) = 2,
    // assert(step = 8) = 3,
    // assert(step = 12) = 4,
    let values = vec![
        BaseElement::new(1),
        BaseElement::new(2),
        BaseElement::new(3),
        BaseElement::new(4),
    ];
    assertions.add_cyclic(0, 0, values.clone()).unwrap();

    // build assertion constraint groups
    let groups = super::build_assertion_constraints(&context, assertions, coeff_prng);

    // make sure only one group and one constraint were created
    assert_eq!(1, groups.len(), "one assertion group should be created");
    assert_eq!(
        1,
        groups[0].constraints().len(),
        "a single constraint should be in the group"
    );

    // both divisor and the constraint should evaluate to 0s at x's corresponding to
    // steps 0, 4, 8, and 12
    let divisor = groups[0].divisor();
    let constraint = &groups[0].constraints()[0];
    for (step, &x) in domain.iter().enumerate() {
        match step {
            0 | 4 | 8 | 12 => {
                let trace_value = values[step / 4];
                assert_eq!(BaseElement::ZERO, divisor.evaluate_at(x));
                assert_eq!(BaseElement::ZERO, constraint.evaluate_at(x, trace_value));
            }
            _ => assert_ne!(BaseElement::ZERO, divisor.evaluate_at(x)),
        }
    }
}

#[test]
fn build_assertion_constraints_many_cyclic_assertions() {
    // set up computation context
    let trace_length = 16;
    let context = build_context(trace_length);
    let coeff_prng = RandomElementGenerator::new([1; 32], 0, blake3);
    let domain = BaseElement::get_power_series(context.generators().trace_domain, trace_length);

    // initialize assertions collection
    let mut assertions = super::Assertions::new(2, trace_length).unwrap();
    let values = vec![
        BaseElement::new(1),
        BaseElement::new(2),
        BaseElement::new(3),
        BaseElement::new(4),
    ];

    // add various constraints to the collection
    assertions.add_cyclic(0, 0, values.clone()).unwrap(); // steps: 0, 4, 8, 12
    assertions.add_cyclic(0, 3, values.clone()).unwrap(); // steps: 3, 7, 11, 15
    assertions.add_cyclic(0, 2, values[..2].to_vec()).unwrap(); // steps: 2, 10
    assertions.add_cyclic(1, 3, values.clone()).unwrap(); // steps: 3, 7, 11, 15
    assertions.add_cyclic(1, 2, values.clone()).unwrap(); // steps: 2, 6, 10, 14
    assertions.add_cyclic(1, 0, values.clone()).unwrap(); // steps: 0, 4, 8, 12

    // build assertion constraint groups
    let groups = super::build_assertion_constraints(&context, assertions, coeff_prng);

    // make sure the constraints were grouped correctly
    assert_eq!(4, groups.len(), "one assertion group should be created");
    assert_eq!(1, groups[0].constraints().len());
    assert_eq!(2, groups[1].constraints().len());
    assert_eq!(1, groups[2].constraints().len());
    assert_eq!(2, groups[3].constraints().len());

    // group 0 for constraints on steps: 2, 10
    let divisor = groups[0].divisor();
    let constraints = groups[0].constraints();
    for (step, &x) in domain.iter().enumerate() {
        match step {
            2 | 10 => {
                let trace_value = values[step / 8];
                assert_eq!(BaseElement::ZERO, divisor.evaluate_at(x));
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[0].evaluate_at(x, trace_value)
                );
            }
            _ => assert_ne!(BaseElement::ZERO, divisor.evaluate_at(x)),
        }
    }

    // group 1 for constraints on steps: 0, 4, 8, 12
    let divisor = groups[1].divisor();
    let constraints = groups[1].constraints();
    for (step, &x) in domain.iter().enumerate() {
        match step {
            0 | 4 | 8 | 12 => {
                let trace_value = values[step / 4];
                assert_eq!(BaseElement::ZERO, divisor.evaluate_at(x));
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[0].evaluate_at(x, trace_value)
                );
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[1].evaluate_at(x, trace_value)
                );
            }
            _ => assert_ne!(BaseElement::ZERO, divisor.evaluate_at(x)),
        }
    }

    // group 2 for constraints on steps: 2, 6, 10, 14
    let divisor = groups[2].divisor();
    let constraints = groups[2].constraints();
    for (step, &x) in domain.iter().enumerate() {
        match step {
            2 | 6 | 10 | 14 => {
                let trace_value = values[step / 4];
                assert_eq!(BaseElement::ZERO, divisor.evaluate_at(x));
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[0].evaluate_at(x, trace_value)
                );
            }
            _ => assert_ne!(BaseElement::ZERO, divisor.evaluate_at(x)),
        }
    }

    // group 3 for constraints on steps: 3, 7, 11, 15
    let divisor = groups[3].divisor();
    let constraints = groups[3].constraints();
    for (step, &x) in domain.iter().enumerate() {
        match step {
            3 | 7 | 11 | 15 => {
                let trace_value = values[step / 4];
                assert_eq!(BaseElement::ZERO, divisor.evaluate_at(x));
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[0].evaluate_at(x, trace_value)
                );
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[1].evaluate_at(x, trace_value)
                );
            }
            _ => assert_ne!(BaseElement::ZERO, divisor.evaluate_at(x)),
        }
    }
}

#[test]
fn build_assertion_constraints_point_and_cyclic_assertions() {
    // set up computation context
    let trace_length = 16;
    let context = build_context(trace_length);
    let coeff_prng = RandomElementGenerator::new([1; 32], 0, blake3);
    let domain = BaseElement::get_power_series(context.generators().trace_domain, trace_length);

    // initialize assertions collection
    let mut assertions = super::Assertions::new(2, trace_length).unwrap();
    let values = vec![
        BaseElement::new(1),
        BaseElement::new(2),
        BaseElement::new(3),
        BaseElement::new(4),
    ];

    // add assertions
    assertions.add_point(0, 1, BaseElement::new(5)).unwrap();
    assertions.add_point(0, 5, BaseElement::new(7)).unwrap();
    assertions.add_point(1, 1, BaseElement::new(9)).unwrap();
    assertions.add_cyclic(0, 0, values.clone()).unwrap();
    assertions.add_cyclic(0, 2, values.clone()).unwrap();
    assertions.add_cyclic(1, 0, values.clone()).unwrap();

    // build assertion constraint groups
    let groups = super::build_assertion_constraints(&context, assertions, coeff_prng);

    // make sure the assertions were grouped correctly
    assert_eq!(4, groups.len());
    assert_eq!(2, groups[0].constraints().len());
    assert_eq!(1, groups[1].constraints().len());
    assert_eq!(2, groups[2].constraints().len());
    assert_eq!(1, groups[3].constraints().len());

    // group 0 for constraints on steps: 1 for registers 0 and 1
    let divisor = groups[0].divisor();
    let constraints = groups[0].constraints();
    for (step, &x) in domain.iter().enumerate() {
        match step {
            1 => {
                assert_eq!(BaseElement::ZERO, divisor.evaluate_at(x));
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[0].evaluate_at(x, BaseElement::new(5))
                );
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[1].evaluate_at(x, BaseElement::new(9))
                );
            }
            _ => assert_ne!(BaseElement::ZERO, divisor.evaluate_at(x)),
        }
    }

    // group 1 for constraints on steps: 5 for registers 0
    let divisor = groups[1].divisor();
    let constraints = groups[1].constraints();
    for (step, &x) in domain.iter().enumerate() {
        match step {
            5 => {
                assert_eq!(BaseElement::ZERO, divisor.evaluate_at(x));
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[0].evaluate_at(x, BaseElement::new(7))
                );
            }
            _ => assert_ne!(BaseElement::ZERO, divisor.evaluate_at(x)),
        }
    }

    // group 2 for constraints on steps: 0, 4, 8, 12 for registers 0 and 1
    let divisor = groups[2].divisor();
    let constraints = groups[2].constraints();
    for (step, &x) in domain.iter().enumerate() {
        match step {
            0 | 4 | 8 | 12 => {
                let trace_value = values[step / 4];
                assert_eq!(BaseElement::ZERO, divisor.evaluate_at(x));
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[0].evaluate_at(x, trace_value)
                );
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[1].evaluate_at(x, trace_value)
                );
            }
            _ => assert_ne!(BaseElement::ZERO, divisor.evaluate_at(x)),
        }
    }

    // group 3 for constraints on steps: 2, 6, 10, 14 for register 0
    let divisor = groups[3].divisor();
    let constraints = groups[3].constraints();
    for (step, &x) in domain.iter().enumerate() {
        match step {
            2 | 6 | 10 | 14 => {
                let trace_value = values[step / 4];
                assert_eq!(BaseElement::ZERO, divisor.evaluate_at(x));
                assert_eq!(
                    BaseElement::ZERO,
                    constraints[0].evaluate_at(x, trace_value)
                );
            }
            _ => assert_ne!(BaseElement::ZERO, divisor.evaluate_at(x)),
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_context(trace_length: usize) -> ComputationContext {
    let ce_blowup_factor = 4;
    let lde_blowup_factor = 16;
    let options = ProofOptions::new(32, lde_blowup_factor, 0, blake3);
    ComputationContext::new(
        2,
        trace_length,
        ce_blowup_factor,
        FieldExtension::None,
        options,
    )
}
