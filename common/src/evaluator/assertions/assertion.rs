use crate::errors::AssertionError;
use math::field::BaseElement;
use std::collections::HashMap;

// TYPES AND INTERFACES
// ================================================================================================

#[derive(Debug, PartialEq)]
pub struct PointAssertion {
    register: usize,
    step: usize,
    value: BaseElement,
}

#[derive(Debug, PartialEq)]
pub struct CyclicAssertion {
    first_step: usize,
    stride: usize,
    values: Vec<BaseElement>,
}

#[derive(Debug, PartialEq)]
pub struct Assertions {
    trace_width: usize,
    trace_length: usize,

    /// point assertions indexed by step
    point_assertions: HashMap<usize, Vec<PointAssertion>>,

    /// cyclic assertions indexed by register
    cyclic_assertions: HashMap<usize, Vec<CyclicAssertion>>,
}

// ASSERTION IMPLEMENTATION
// ================================================================================================

impl Assertions {
    /// Returns a new empty assertions collection for an execution trace with the specified width
    /// and length.
    ///
    /// Will return an error if:
    /// * trace width is 0;
    /// * trace length is not a power of two.
    pub fn new(trace_width: usize, trace_length: usize) -> Result<Self, AssertionError> {
        // make sure trace width and length are valid
        if trace_width == 0 {
            return Err(AssertionError::TraceWidthTooShort);
        }
        if !trace_length.is_power_of_two() {
            return Err(AssertionError::TraceLengthNotPowerOfTwo(trace_length));
        }
        // create and return Assertions collection
        Ok(Assertions {
            trace_width,
            trace_length,
            point_assertions: HashMap::new(),
            cyclic_assertions: HashMap::new(),
        })
    }

    /// Adds an assertion that in a valid execution trace, the specified `register` at the
    /// specified `step` must be equal to the specified `value`.
    ///
    /// Returns an error if:
    /// * Register index is greater than the width of the execution trace;
    /// * Step is greater than the length of the execution trace;
    /// * Assertion for the same register and step has already been made, even if the values
    ///   for both assertions are the same.
    pub fn add_point(
        &mut self,
        register: usize,
        step: usize,
        value: BaseElement,
    ) -> Result<(), AssertionError> {
        // make sure register and step are within bounds
        if register >= self.trace_width {
            return Err(AssertionError::InvalidAssertionRegisterIndex(register));
        }
        if step >= self.trace_length {
            return Err(AssertionError::InvalidAssertionStep(step));
        }

        // check if the assertion is covered by any of the cyclic assertions for the same register
        if self.cyclic_assertions.contains_key(&register) {
            for cyclic_assertion in &self.cyclic_assertions[&register] {
                if is_covered_by_cyclic_assertion(step, cyclic_assertion) {
                    return Err(AssertionError::AssertionCoveredByCyclicAssertion(
                        register, step,
                    ));
                }
            }
        }

        // create the assertion
        let assertion = PointAssertion {
            register,
            step,
            value,
        };

        // get the list of point existing assertions for the specified step
        let assertions = self.point_assertions.entry(step).or_default();

        // add assertion to the list using binary search; this makes sure that
        // assertions are always sorted in consistent order (by register index)
        match assertions.binary_search_by_key(&register, |e| e.register) {
            Ok(_) => Err(AssertionError::DuplicateAssertion(register, step)),
            Err(pos) => {
                assertions.insert(pos, assertion);
                Ok(())
            }
        }
    }

    /// Adds a cyclic assertion to the specified register. If `values` contains
    /// only a single value, this is equivalent to creating a point assertion.
    ///
    /// Returns an error if:
    /// * Register index is greater than the width of the execution trace;
    /// * Number of values is not a power of two;
    /// * Number of values is greater than or equal to the trace length;
    /// * First step comes after the end of the cycle implied by the values list;
    /// * Cyclic assertion overlaps with any of the previously added assertions.
    pub fn add_cyclic(
        &mut self,
        register: usize,
        first_step: usize,
        values: Vec<BaseElement>,
    ) -> Result<(), AssertionError> {
        // make sure the register is valid
        if register >= self.trace_width {
            return Err(AssertionError::InvalidAssertionRegisterIndex(register));
        }
        // if there is only one value in the list, convert it to a point assertion
        if values.len() == 1 {
            return self.add_point(register, first_step, values[0]);
        }
        // make sure the number of values is power of 2
        if !values.len().is_power_of_two() {
            return Err(AssertionError::NumberOfValuesNotPowerOfTwo(values.len()));
        }
        // make sure there aren't to many values
        if values.len() >= self.trace_length {
            return Err(AssertionError::TooManyCyclicAssertionValues(values.len()));
        }

        // determine cycle length; this will always divide evenly since both trace length
        // and number of values are powers of two and trace_length > values.lne()
        let stride = self.trace_length / values.len();
        // make sure the fist step falls within the first cycle
        if first_step >= stride {
            return Err(AssertionError::InvalidFirstCycleStart(first_step, stride));
        }

        // create the assertion
        let assertion = CyclicAssertion {
            first_step,
            stride,
            values,
        };
        // check if it overlaps with any of the existing cyclic assertions for the same register
        let assertions = self.cyclic_assertions.entry(register).or_default();
        is_overlapping_with_cycles(&assertion, assertions)?;
        // check if it overlaps with any of the point assertions for the same register
        for assertions in self.point_assertions.values() {
            if let Ok(pos) = assertions.binary_search_by_key(&register, |e| e.register) {
                if is_covered_by_cyclic_assertion(assertions[pos].step, &assertion) {
                    return Err(AssertionError::CoveringCyclicAssertion(first_step, stride));
                }
            }
        }

        // add assertion to the list of cycles for the register and return
        assertions.push(assertion);
        Ok(())
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Returns true if the specified step is covered by any of the steps in the cyclic assertion.
fn is_covered_by_cyclic_assertion(step: usize, assertion: &CyclicAssertion) -> bool {
    if step == assertion.first_step {
        return true;
    } else if step > assertion.first_step {
        let delta = step - assertion.first_step;
        if delta % assertion.stride == 0 {
            return true;
        }
    }
    false
}

/// Checks if the provided cyclic assertion overlaps with any of the other cyclic assertions
fn is_overlapping_with_cycles(
    assertion: &CyclicAssertion,
    assertions: &[CyclicAssertion],
) -> Result<(), AssertionError> {
    for cycle in assertions {
        if cycle.first_step == assertion.first_step {
            return Err(AssertionError::OverlappingCyclicAssertion(
                assertion.first_step,
                assertion.stride,
            ));
        } else if cycle.stride != assertion.stride {
            let (start, end, stride) = if cycle.stride < assertion.stride {
                let end = if cycle.first_step > assertion.first_step {
                    assertion.first_step + assertion.stride
                } else {
                    assertion.first_step
                };
                (cycle.first_step, end, cycle.stride)
            } else {
                let end = if assertion.first_step > cycle.first_step {
                    cycle.first_step + cycle.stride
                } else {
                    cycle.first_step
                };
                (assertion.first_step, end, assertion.stride)
            };
            if (end - start) % stride == 0 {
                return Err(AssertionError::OverlappingCyclicAssertion(
                    assertion.first_step,
                    assertion.stride,
                ));
            }
        }
    }

    Ok(())
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {

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
}
