use std::vec;

use crate::{errors::AssertionError, ComputationContext, ConstraintDivisor};
use crypto::RandomElementGenerator;
use math::{
    fft,
    field::{BaseElement, FieldElement},
};

mod constraints;
pub use constraints::{AssertionConstraint, AssertionConstraintGroup};

mod assertion;
pub use assertion::Assertion;

#[cfg(test)]
mod tests;

// TYPES AND INTERFACES
// ================================================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Assertions {
    trace_width: usize,
    trace_length: usize,
    assertions: Vec<Assertion>,
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
            assertions: Vec::new(),
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
        self.assertions.is_empty()
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Executes the provided closure for all assertions in this collection.
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, usize, BaseElement),
    {
        for assertion in self.assertions.iter() {
            for i in 0..assertion.num_values {
                let value_index = if assertion.values.len() == 1 { 0 } else { i };
                f(
                    assertion.register,
                    assertion.first_step + assertion.stride * i,
                    assertion.values[value_index],
                );
            }
        }
    }

    // ASSERTION ADDITION METHODS
    // --------------------------------------------------------------------------------------------

    /// Adds a new assertion to the collection. This method is not intended to be used directly;
    /// it is invoked from add_single(), add_cyclic(), and add_list() methods. These methods
    /// should be used instead.
    pub fn add(&mut self, assertion: Assertion) -> Result<(), AssertionError> {
        // make sure register and step are within bounds
        if assertion.register >= self.trace_width {
            return Err(AssertionError::InvalidAssertionRegisterIndex(
                assertion.register,
            ));
        }
        if assertion.trace_length() != self.trace_length {
            return Err(AssertionError::InvalidAssertionTraceLength(
                assertion.trace_length(),
                self.trace_length,
            ));
        }

        // check if it overlaps with any of the assertions already in the collection
        for a in self
            .assertions
            .iter()
            .filter(|a| a.register == assertion.register)
        {
            if a.overlaps_with(&assertion) {
                return Err(AssertionError::DuplicateAssertion(assertion));
            }
        }

        // add assertion to the list using binary search; this makes sure that assertions
        // are always sorted in consistent order (first by stride and then by first_step)
        match self.assertions.binary_search(&assertion) {
            Ok(pos) | Err(pos) => self.assertions.insert(pos, assertion),
        }

        Ok(())
    }

    /// Adds an assertion that in a valid execution trace, the specified `register` at the
    /// specified `step` must be equal to the specified `value`.
    ///
    /// Returns an error if:
    /// * Register index is greater than the width of the execution trace;
    /// * Step is greater than the length of the execution trace;
    /// * Assertion for the same register and step has already been made, even if the values
    ///   for both assertions are the same.
    pub fn add_single(
        &mut self,
        register: usize,
        step: usize,
        value: BaseElement,
    ) -> Result<(), AssertionError> {
        // create the assertion and add it to the collection
        let assertion = Assertion::single(register, step, value, self.trace_length)?;
        self.add(assertion)
    }

    /// Adds an assertion that in a valid execution trace the specified `register` must be equal
    /// to the value at steps which start with `first_step` and repeat in equal intervals
    /// `num_values` number of times.
    ///
    /// Returns an error if:
    /// * Register index is greater than the width of the execution trace;
    /// * Number of values is not a power of two;
    /// * Number of values is zero or is greater than or equal to the trace length;
    /// * First step comes after the end of the interval implied by the `num_values` parameter;
    /// * Assertion overlaps with any of the previously added assertions.
    pub fn add_cyclic(
        &mut self,
        register: usize,
        first_step: usize,
        num_values: usize,
        value: BaseElement,
    ) -> Result<(), AssertionError> {
        // create the assertion and add it to the collection
        let assertion =
            Assertion::cyclic(register, first_step, num_values, value, self.trace_length)?;
        self.add(assertion)
    }

    /// Adds a list of assertions to the specified register. The asserted values are assumed
    /// to be spaced in equal intervals which have a length of some power of two.
    ///
    /// Returns an error if:
    /// * Register index is greater than the width of the execution trace;
    /// * Number of values is not a power of two;
    /// * Number of values is zero or is greater than or equal to the trace length;
    /// * First step comes after the end of the interval implied by the values list;
    /// * Assertion overlaps with any of the previously added assertions.
    pub fn add_list(
        &mut self,
        register: usize,
        first_step: usize,
        values: Vec<BaseElement>,
    ) -> Result<(), AssertionError> {
        // create the assertion and add it to the collection
        let assertion = Assertion::list(register, first_step, values, self.trace_length)?;
        self.add(assertion)
    }

    // CONSTRAINTS
    // --------------------------------------------------------------------------------------------

    /// Converts this assertion collection into assertion constraints grouped by common divisor.
    pub fn into_constraints(
        self,
        context: &ComputationContext,
        mut coeff_prng: RandomElementGenerator,
    ) -> Vec<AssertionConstraintGroup> {
        // group assertions by step - i.e.: assertions for the first step are grouped together,
        // assertions for the last step are grouped together etc.
        let mut groups: Vec<AssertionConstraintGroup> = Vec::new();

        // compute inverse of the trace domain generator; this will be used for offset
        // computations when creating a new constraint
        let inv_g = context.generators().trace_domain.inv();

        // set up variables to track values from the previous iteration of the loop
        let mut stride = usize::MAX;
        let mut first_step = usize::MAX;
        let mut inv_twiddles = Vec::new();

        // iterate over all assertions, which are sorted first by stride and then by first_step
        // in ascending order
        for assertion in self.assertions {
            if assertion.stride != stride {
                // when strides change, we need to build new inv_twiddles and also
                // start a new assertion group
                stride = assertion.stride;
                first_step = assertion.first_step;

                // if an assertion consists of two values or more, we'll need to interpolate
                // an assertion polynomial from these values; for that, we'll need twiddles
                if assertion.num_values > 1 {
                    inv_twiddles = fft::get_inv_twiddles(assertion.num_values);
                }
                groups.push(AssertionConstraintGroup::new(
                    context,
                    ConstraintDivisor::from_assertion(&assertion, &context),
                ));
            } else if assertion.first_step != first_step {
                // if only the first_step changed, we can use inv_twiddles from the previous
                // iteration, but we do need to start a new assertion group
                first_step = assertion.first_step;
                groups.push(AssertionConstraintGroup::new(
                    context,
                    ConstraintDivisor::from_assertion(&assertion, &context),
                ));
            }

            // add a new assertion constraint to the current group (last group in the list)
            let constraint = assertion.into_constraint(inv_g, &inv_twiddles, &mut coeff_prng);
            groups.last_mut().unwrap().add_constraint(constraint);
        }

        // make sure groups are sorted by adjustment degree
        groups.sort_by_key(|c| c.degree_adjustment);

        groups
    }
}

impl IntoIterator for Assertions {
    type Item = Assertion;
    type IntoIter = vec::IntoIter<Assertion>;

    /// Converts this collection into an iterator over all assertions.
    fn into_iter(self) -> Self::IntoIter {
        self.assertions.into_iter()
    }
}
