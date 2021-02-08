use super::*;
use math::field::FieldElement;

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
