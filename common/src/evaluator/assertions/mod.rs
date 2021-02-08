use crate::errors::AssertionError;
use math::field::BaseElement;
use std::collections::{btree_map, BTreeMap};

mod constraints;
pub use constraints::{build_assertion_constraints, AssertionConstraint, AssertionConstraintGroup};

#[cfg(test)]
mod tests;

// TYPES AND INTERFACES
// ================================================================================================

/// Asserts that a valid execution trace must have the specified value in the specified register
/// at the specified step.
#[derive(Debug, Clone, PartialEq)]
pub struct PointAssertion {
    pub register: usize,
    pub step: usize,
    pub value: BaseElement,
}

/// Asserts that a valid execution trace must have the specified values appear in the the
/// specified register at the intervals specified by the stride. For example, for first_step = 1
/// and stride = 4, the asserted steps would be 1, 5, 9, 13 etc.
#[derive(Debug, Clone, PartialEq)]
pub struct CyclicAssertion {
    pub register: usize,
    pub first_step: usize,
    pub stride: usize,
    pub values: Vec<BaseElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assertions {
    trace_width: usize,
    trace_length: usize,

    /// point assertions indexed by step
    point_assertions: BTreeMap<usize, Vec<PointAssertion>>,

    /// cyclic assertions indexed by register
    cyclic_assertions: BTreeMap<usize, Vec<CyclicAssertion>>,
}

// ASSERTIONS IMPLEMENTATION
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
            point_assertions: BTreeMap::new(),
            cyclic_assertions: BTreeMap::new(),
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns execution trace width for this assertion collection.
    pub fn trace_width(&self) -> usize {
        self.trace_width
    }

    /// Returns execution trace length for this assertion collection.
    pub fn trace_length(&self) -> usize {
        self.trace_length
    }

    /// Returns true if this assertion collection does not contain any assertions.
    pub fn is_empty(&self) -> bool {
        self.point_assertions.is_empty() && self.cyclic_assertions.is_empty()
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over point assertions grouped by step.
    pub fn point_assertions(&self) -> btree_map::Iter<usize, Vec<PointAssertion>> {
        self.point_assertions.iter()
    }

    /// Returns an iterator over cyclic assertions grouped by register.
    pub fn cyclic_assertions(&self) -> btree_map::Iter<usize, Vec<CyclicAssertion>> {
        self.cyclic_assertions.iter()
    }

    /// Executes the provided closure for all assertions in this collection.
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, usize, BaseElement),
    {
        // iterate over all point assertions
        for assertions in self.point_assertions.values() {
            for assertion in assertions {
                f(assertion.register, assertion.step, assertion.value)
            }
        }

        // iterate over all instances of cyclic assertions
        for assertions in self.cyclic_assertions.values() {
            for assertion in assertions {
                for (i, &value) in assertion.values.iter().enumerate() {
                    f(
                        assertion.register,
                        assertion.first_step + assertion.stride * i,
                        value,
                    );
                }
            }
        }
    }

    // ASSERTION ADDITION METHODS
    // --------------------------------------------------------------------------------------------

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
            register,
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
#[allow(clippy::comparison_chain)]
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
