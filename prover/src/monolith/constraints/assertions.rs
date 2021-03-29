use common::ComputationContext;
use math::{
    fft,
    field::{BaseElement, FieldElement},
};
use std::collections::HashMap;

// CONSTANTS
// ================================================================================================

/// Assertion polynomials with this degree or smaller will be evaluated on the fly, while for
/// larger polynomials all evaluations over the constraint evaluation domain will be pre-computed.
const SMALL_POLY_DEGREE: usize = 63;

// ASSERTION CONSTRAINT GROUP
// ================================================================================================

/// Contains constraints all having the same divisor. The constraints are separated into single
/// value constraints, small polynomial constraints, and large polynomial constraints.
pub struct AssertionConstraintGroup {
    pub(super) degree_adjustment: u32,
    single_value_constraints: Vec<SingleValueConstraint>,
    small_poly_constraints: Vec<SmallPolyConstraint>,
    large_poly_constraints: Vec<LargePolyConstraint>,
}

impl AssertionConstraintGroup {
    /// Creates a new specialized constraint group; twiddles and ce_blowup_factor are passed in for
    /// evaluating large polynomial constraints (if any).
    pub fn new(
        group: common::AssertionConstraintGroup,
        context: &ComputationContext,
        twiddle_map: &mut HashMap<usize, Vec<BaseElement>>,
    ) -> AssertionConstraintGroup {
        let mut result = AssertionConstraintGroup {
            degree_adjustment: group.degree_adjustment(),
            single_value_constraints: Vec::new(),
            small_poly_constraints: Vec::new(),
            large_poly_constraints: Vec::new(),
        };

        for constraint in group.constraints() {
            if constraint.poly().len() == 1 {
                result.single_value_constraints.push(SingleValueConstraint {
                    register: constraint.register(),
                    value: constraint.poly()[0],
                    coefficients: *constraint.cc(),
                });
            } else if constraint.poly().len() < SMALL_POLY_DEGREE {
                result.small_poly_constraints.push(SmallPolyConstraint {
                    register: constraint.register(),
                    poly: constraint.poly().to_vec(),
                    x_offset: constraint.x_offset(),
                    coefficients: *constraint.cc(),
                });
            } else {
                // evaluate the polynomial over the entire constraint evaluation domain; first
                // get twiddles for the evaluation; if twiddles haven't been built yet, build them
                let poly_length = constraint.poly().len();
                let twiddles = twiddle_map
                    .entry(poly_length)
                    .or_insert_with(|| fft::get_twiddles(poly_length));

                let values = fft::evaluate_poly_with_offset(
                    constraint.poly(),
                    &twiddles,
                    context.domain_offset(),
                    context.ce_domain_size() / poly_length,
                );

                result.large_poly_constraints.push(LargePolyConstraint {
                    register: constraint.register(),
                    values,
                    step_offset: constraint.step_offset() * context.ce_blowup_factor(),
                    coefficients: *constraint.cc(),
                });
            }
        }

        result
    }

    /// Evaluates the constraints contained in this group at the specified step of the
    /// execution trace.
    pub fn evaluate<E: FieldElement + From<BaseElement>>(
        &self,
        state: &[E],
        ce_step: usize,
        x: E,
        xp: E,
    ) -> E {
        let mut result = E::ZERO;

        // evaluate all single-value constraints
        for constraint in self.single_value_constraints.iter() {
            result += constraint.evaluate(state, xp);
        }

        // evaluate all small polynomial constraints
        for constraint in self.small_poly_constraints.iter() {
            result += constraint.evaluate(state, x, xp);
        }

        // evaluate all large polynomial constraints
        for constraint in self.large_poly_constraints.iter() {
            result += constraint.evaluate(state, ce_step, xp);
        }

        result
    }
}

// CONSTRAINT SPECIALIZATIONS
// ================================================================================================

/// A constraint where the numerator can be represented by p(x) - v, where v is the asserted value,
/// and p(x) is the trace polynomial for the register against which the constraint is applied.
struct SingleValueConstraint {
    register: usize,
    value: BaseElement,
    coefficients: (BaseElement, BaseElement),
}

impl SingleValueConstraint {
    pub fn evaluate<E: FieldElement + From<BaseElement>>(&self, state: &[E], xp: E) -> E {
        let evaluation = state[self.register] - E::from(self.value);
        evaluation * (E::from(self.coefficients.0) + E::from(self.coefficients.1) * xp)
    }
}

/// A constraint where the numerator can be represented by p(x) - c(x), where c(x) is the
/// polynomial describing a set of asserted values. This specialization is useful when the
// degree of c(x) is relatively small, and thus, is cheap to evaluate on the fly.
struct SmallPolyConstraint {
    register: usize,
    poly: Vec<BaseElement>,
    x_offset: BaseElement,
    coefficients: (BaseElement, BaseElement),
}

impl SmallPolyConstraint {
    pub fn evaluate<E: FieldElement + From<BaseElement>>(&self, state: &[E], x: E, xp: E) -> E {
        let x = x * E::from(self.x_offset);
        // evaluate constraint polynomial as x * offset using Horner evaluation
        let assertion_value = self
            .poly
            .iter()
            .rev()
            .fold(E::ZERO, |result, coeff| result * x + E::from(*coeff));
        let evaluation = state[self.register] - assertion_value;
        evaluation * (E::from(self.coefficients.0) + E::from(self.coefficients.1) * xp)
    }
}

/// A constraint where the numerator can be represented by p(x) - c(x), where c(x) is a large
/// polynomial. In such cases, we pre-compute evaluations of c(x) by evaluating it over the
/// entire constraint evaluation domain (using FFT).
struct LargePolyConstraint {
    register: usize,
    values: Vec<BaseElement>,
    step_offset: usize,
    coefficients: (BaseElement, BaseElement),
}

impl LargePolyConstraint {
    pub fn evaluate<E: FieldElement + From<BaseElement>>(
        &self,
        state: &[E],
        ce_step: usize,
        xp: E,
    ) -> E {
        let value_index = if self.step_offset > 0 {
            // if the assertion happens on steps which are not a power of 2, we need to offset the
            // evaluation; the below basically computes (ce_step - step_offset) % values.len();
            // this is equivalent to evaluating the polynomial at x * x_offset coordinate.
            if self.step_offset > ce_step {
                self.values.len() + ce_step - self.step_offset
            } else {
                ce_step - self.step_offset
            }
        } else {
            ce_step
        };
        let evaluation = state[self.register] - E::from(self.values[value_index]);
        evaluation * (E::from(self.coefficients.0) + E::from(self.coefficients.1) * xp)
    }
}
