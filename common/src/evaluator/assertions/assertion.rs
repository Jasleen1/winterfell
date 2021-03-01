use crate::{errors::AssertionError, AssertionConstraint};
use crypto::RandomElementGenerator;
use math::{
    fft,
    field::{BaseElement, FieldElement},
};
use std::{
    cmp::{Ord, Ordering, PartialOrd},
    fmt::{Display, Formatter},
};

// ASSERTION
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assertion {
    pub(crate) register: usize,
    pub(crate) first_step: usize,
    pub(crate) stride: usize,
    pub(crate) num_values: usize,
    pub(crate) values: Vec<BaseElement>,
}

impl Assertion {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns an assertion requiring that the value in the specified register at the specified
    /// step must be equal to the provided value.
    pub fn single(
        register: usize,
        step: usize,
        value: BaseElement,
        trace_length: usize,
    ) -> Result<Self, AssertionError> {
        if step >= trace_length {
            return Err(AssertionError::InvalidAssertionStep(step, trace_length));
        }
        Ok(Assertion {
            register,
            first_step: step,
            stride: trace_length,
            num_values: 1,
            values: vec![value],
        })
    }

    /// Returns an assertion requiring that values in the specified `register` must be equal to
    /// the specified `value` at the steps which start at `first_step` and repeat in equal
    /// intervals `num_values` number of times until `trace_length` is reached.
    pub fn cyclic(
        register: usize,
        first_step: usize,
        num_values: usize,
        value: BaseElement,
        trace_length: usize,
    ) -> Result<Self, AssertionError> {
        check_num_asserted_values(num_values, trace_length)?;
        let stride = trace_length / num_values;
        if first_step >= stride {
            return Err(AssertionError::InvalidAssertionStep(first_step, stride));
        }
        Ok(Assertion {
            register,
            first_step,
            stride,
            num_values,
            values: vec![value],
        })
    }

    /// Returns an assertion requiring that values in the specified `register` must be equal to
    /// the provided `values` at steps which start at `first_step` and repeat in equal intervals
    /// until all values have been consumed.
    pub fn list(
        register: usize,
        first_step: usize,
        values: Vec<BaseElement>,
        trace_length: usize,
    ) -> Result<Self, AssertionError> {
        let num_values = values.len();
        check_num_asserted_values(num_values, trace_length)?;
        let stride = trace_length / num_values;
        if first_step >= stride {
            return Err(AssertionError::InvalidAssertionStep(first_step, stride));
        }
        Ok(Assertion {
            register,
            first_step,
            stride,
            num_values: values.len(),
            values,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    pub fn register(&self) -> usize {
        self.register
    }

    pub fn first_step(&self) -> usize {
        self.first_step
    }

    pub fn values(&self) -> &[BaseElement] {
        &self.values
    }

    pub fn trace_length(&self) -> usize {
        self.num_values * self.stride
    }

    // PUBLIC METHODS
    // --------------------------------------------------------------------------------------------

    /// Checks if this assertion overlaps with the provided assertion. Overlap is defined as
    /// asserting a value for the same step in the same register.
    pub fn overlaps_with(&self, other: &Assertion) -> bool {
        if self.register != other.register {
            return false;
        }
        if self.first_step == other.first_step {
            return true;
        }
        if self.stride == other.stride {
            return false;
        }
        // at this point we know that assertions are for the same register but they start
        // on different steps and also have different strides
        let (start, end, stride) = if self.stride < other.stride {
            let end = if self.first_step > other.first_step {
                other.first_step + other.stride
            } else {
                other.first_step
            };
            (self.first_step, end, self.stride)
        } else {
            let end = if other.first_step > self.first_step {
                self.first_step + self.stride
            } else {
                self.first_step
            };
            (other.first_step, end, other.stride)
        };

        (end - start) % stride == 0
    }

    /// Transforms this assertion into the numerator portion of a constraint.
    pub fn into_constraint(
        self,
        inv_g: BaseElement,
        inv_twiddles: &[BaseElement],
        coeff_prng: &mut RandomElementGenerator,
    ) -> AssertionConstraint {
        // build a polynomial which evaluates to constraint values at asserted steps; for
        // single-value assertions we use the value as constant coefficient of degree 0
        // polynomial; but if there is more than one value, we need to interpolate them into
        // a polynomial using inverse FFT
        let mut x_offset = BaseElement::ONE;
        let mut poly = self.values;
        if poly.len() > 1 {
            fft::interpolate_poly(&mut poly, &inv_twiddles, true);
            if self.first_step != 0 {
                // if the assertions don't fall on the steps which are powers of two, we can't
                // use FFT to interpolate the values into a polynomial. This would make such
                // assertions quite impractical. To get around this, we still use FFT to build
                // the polynomial, but then we evaluate it as f(x * offset) instead of f(x)
                x_offset = inv_g.exp((self.first_step as u64).into());
            }
        }

        AssertionConstraint {
            register: self.register,
            poly,
            x_offset,
            step_offset: self.first_step,
            cc: coeff_prng.draw_pair(),
        }
    }
}

// OTHER TRAIT IMPLEMENTATIONS
// =================================================================================================

/// We define ordering of assertions to be first by stride, then by first_step, and finally by
/// register in ascending order.
impl Ord for Assertion {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.stride == other.stride {
            if self.first_step == other.first_step {
                self.register.partial_cmp(&other.register).unwrap()
            } else {
                self.first_step.partial_cmp(&other.first_step).unwrap()
            }
        } else {
            self.stride.partial_cmp(&other.stride).unwrap()
        }
    }
}

impl PartialOrd for Assertion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Assertion {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "(register={}, ", self.register)?;
        match self.num_values {
            1 => write!(f, "step={}, ", self.first_step)?,
            2 => {
                let second_step = self.first_step + self.stride;
                write!(f, "steps=[{}, {}], ", self.first_step, second_step)?;
            }
            _ => {
                let second_step = self.first_step + self.stride;
                let last_step = (self.num_values - 1) * self.stride + self.first_step;
                write!(
                    f,
                    "steps=[{}, {}, ..., {}], ",
                    self.first_step, second_step, last_step
                )?;
            }
        }
        match self.values.len() {
            1 => write!(f, "value={})", self.values[0]),
            2 => write!(f, "values=[{}, {}])", self.values[0], self.values[1]),
            _ => write!(f, "values=[{}, {}, ...])", self.values[0], self.values[1]),
        }
    }
}

// HELPER FUNCTIONS
// =================================================================================================
fn check_num_asserted_values(num_values: usize, trace_length: usize) -> Result<(), AssertionError> {
    if num_values >= trace_length {
        return Err(AssertionError::TooManyAssertedValues(
            num_values,
            trace_length,
        ));
    }
    if num_values == 0 {
        return Err(AssertionError::ZeroAssertedValues);
    }
    if !num_values.is_power_of_two() {
        return Err(AssertionError::AssertedValuesNotPowerOfTwo(num_values));
    }
    Ok(())
}
