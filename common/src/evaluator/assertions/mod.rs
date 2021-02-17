use crate::errors::AssertionError;
use math::field::BaseElement;
use std::cmp::Ordering;

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

    /// point assertions sorted first by step and then by register.
    point_assertions: Vec<PointAssertion>,

    /// cyclic assertions sorted first by stride and then by first_step.
    cyclic_assertions: Vec<CyclicAssertion>,
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
            point_assertions: Vec::new(),
            cyclic_assertions: Vec::new(),
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

    /// Returns list of point assertions sorted first by step and then by register.
    pub fn point_assertions(&self) -> &[PointAssertion] {
        &self.point_assertions
    }

    /// Returns a list of cyclic assertions sorted first by stride and then by first_step.
    pub fn cyclic_assertions(&self) -> &[CyclicAssertion] {
        &self.cyclic_assertions
    }

    /// Executes the provided closure for all assertions in this collection.
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, usize, BaseElement),
    {
        // iterate over all point assertions
        for assertion in self.point_assertions.iter() {
            f(assertion.register, assertion.step, assertion.value)
        }

        // iterate over all instances of cyclic assertions
        for assertion in self.cyclic_assertions.iter() {
            for (i, &value) in assertion.values.iter().enumerate() {
                f(
                    assertion.register,
                    assertion.first_step + assertion.stride * i,
                    value,
                );
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
        for cyclic_assertion in self
            .cyclic_assertions
            .iter()
            .filter(|a| a.register == register)
        {
            if is_covered_by_cyclic_assertion(step, cyclic_assertion) {
                return Err(AssertionError::AssertionCoveredByCyclicAssertion(
                    register, step,
                ));
            }
        }

        // create the assertion
        let assertion = PointAssertion {
            register,
            step,
            value,
        };

        // add assertion to the list using binary search; this makes sure that assertions
        // are always sorted in consistent order (first by step and then by register index)
        match self
            .point_assertions
            .binary_search_by(|a| point_assertion_comparator(a, step, register))
        {
            Ok(_) => Err(AssertionError::DuplicateAssertion(register, step)),
            Err(pos) => {
                self.point_assertions.insert(pos, assertion);
                Ok(())
            }
        }
    }

    pub fn add_cyclic_value(
        &mut self,
        register: usize,
        first_step: usize,
        num_cycles: usize,
        value: BaseElement,
    ) -> Result<(), AssertionError> {
        // make sure the register index is valid
        if register >= self.trace_width {
            return Err(AssertionError::InvalidAssertionRegisterIndex(register));
        }

        let stride = self.trace_length / num_cycles;

        if !stride.is_power_of_two() {}

        if stride > self.trace_length {}

        // create the assertion
        let assertion = CyclicAssertion {
            register,
            first_step,
            stride,
            values: vec![value],
        };

        // check if it overlaps with any of the existing cyclic assertions for the same register
        for a in self
            .cyclic_assertions
            .iter()
            .filter(|a| a.register == register)
        {
            if are_cyclic_assertions_overlapping(a, &assertion) {
                return Err(AssertionError::OverlappingCyclicAssertion(
                    assertion.first_step,
                    assertion.stride,
                ));
            }
        }

        // check if it overlaps with any of the point assertions for the same register
        for point_assertion in self
            .point_assertions
            .iter()
            .filter(|a| a.register == register)
        {
            if is_covered_by_cyclic_assertion(point_assertion.step, &assertion) {
                return Err(AssertionError::CoveringCyclicAssertion(first_step, stride));
            }
        }

        // add assertion to the list in the position required to ensure that cyclic assertions
        // are sorted by stride and first_step
        match self
            .cyclic_assertions
            .binary_search_by(|a| cyclic_assertion_comparator(a, stride, first_step))
        {
            Ok(pos) | Err(pos) => self.cyclic_assertions.insert(pos, assertion),
        }

        Ok(())
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
        for a in self
            .cyclic_assertions
            .iter()
            .filter(|a| a.register == register)
        {
            if are_cyclic_assertions_overlapping(a, &assertion) {
                return Err(AssertionError::OverlappingCyclicAssertion(
                    assertion.first_step,
                    assertion.stride,
                ));
            }
        }

        // check if it overlaps with any of the point assertions for the same register
        for point_assertion in self
            .point_assertions
            .iter()
            .filter(|a| a.register == register)
        {
            if is_covered_by_cyclic_assertion(point_assertion.step, &assertion) {
                return Err(AssertionError::CoveringCyclicAssertion(first_step, stride));
            }
        }

        // add assertion to the list in the position required to ensure that cyclic assertions
        // are sorted by stride and first_step
        match self
            .cyclic_assertions
            .binary_search_by(|a| cyclic_assertion_comparator(a, stride, first_step))
        {
            Ok(pos) | Err(pos) => self.cyclic_assertions.insert(pos, assertion),
        }
        Ok(())
    }

    // DESTRUCTURING
    // --------------------------------------------------------------------------------------------

    /// Destructures this assertion collection into vectors of assertions.
    pub fn into_lists(self) -> (Vec<PointAssertion>, Vec<CyclicAssertion>) {
        (self.point_assertions, self.cyclic_assertions)
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

/// Checks if the provided cyclic assertions overlaps with each other.
fn are_cyclic_assertions_overlapping(a: &CyclicAssertion, b: &CyclicAssertion) -> bool {
    if a.first_step == b.first_step {
        return true;
    } else if a.stride != b.stride {
        let (start, end, stride) = if a.stride < b.stride {
            let end = if a.first_step > b.first_step {
                b.first_step + b.stride
            } else {
                b.first_step
            };
            (a.first_step, end, a.stride)
        } else {
            let end = if b.first_step > a.first_step {
                a.first_step + a.stride
            } else {
                a.first_step
            };
            (b.first_step, end, b.stride)
        };
        if (end - start) % stride == 0 {
            return true;
        }
    }
    false
}

fn point_assertion_comparator(
    assertion: &PointAssertion,
    step: usize,
    register: usize,
) -> Ordering {
    if assertion.step == step {
        assertion.register.partial_cmp(&register).unwrap()
    } else {
        assertion.step.partial_cmp(&step).unwrap()
    }
}

fn cyclic_assertion_comparator(
    assertion: &CyclicAssertion,
    stride: usize,
    first_step: usize,
) -> Ordering {
    if assertion.stride == stride {
        assertion.first_step.partial_cmp(&first_step).unwrap()
    } else {
        assertion.stride.partial_cmp(&stride).unwrap()
    }
}
