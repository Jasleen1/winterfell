use common::{ConstraintDivisor, ConstraintEvaluator, TransitionEvaluator};
use math::{
    fft,
    field::{BaseElement, FieldElement},
};

// CONSTANTS
// ================================================================================================

/// Assertion polynomials with this degree or smaller will be evaluated on the fly, while for
/// larger polynomials all evaluations over the constraint evaluation domain will be pre-computed.
const SMALL_POLY_DEGREE: usize = 63;

// CONSTANTS
// ================================================================================================

/// Converts assertion constraints to a specialized representation. This is especially important
/// for large polynomial constraints. Such constraints are evaluated over the entire constraint
/// evaluation domain and the results are cached. This allows us to replace polynomial evaluation
/// at every step with a single lookup.
pub fn prepare_assertion_constraints<T: TransitionEvaluator>(
    evaluator: &ConstraintEvaluator<T>,
    divisors: &mut Vec<ConstraintDivisor>,
) -> Vec<AssertionConstraintGroup> {
    let mut twiddles = Vec::new();
    let mut groups = Vec::with_capacity(evaluator.assertion_constraints().len());
    for group in evaluator.assertion_constraints() {
        // if the group contains large polynomial constraints, and we haven't built twiddles yet
        // build them
        if group.max_poly_degree() >= SMALL_POLY_DEGREE && twiddles.is_empty() {
            twiddles =
                fft::get_twiddles(evaluator.ce_domain_generator(), evaluator.ce_domain_size());
        }
        groups.push(AssertionConstraintGroup::new(
            group,
            &twiddles,
            evaluator.ce_blowup_factor(),
        ));
        divisors.push(group.divisor().clone());
    }
    groups
}

/// Evaluates all assertion group at a specific step of the execution trace. `ce_step` is the
/// step in the constraint evaluation domain and `x` is the corresponding domain value. That is
/// x = g^ce_step, where g is the generator of the constraint evaluation domain.
pub fn evaluate_assertions<E: FieldElement + From<BaseElement>>(
    constraint_groups: &[AssertionConstraintGroup],
    state: &[E],
    x: E,
    ce_step: usize,
    result: &mut Vec<E>,
) {
    // compute the adjustment degree outside of the group so that we can re-use
    // it for groups which have the same adjustment degree
    let mut degree_adjustment = constraint_groups[0].degree_adjustment;
    let mut xp = x.exp(degree_adjustment.into());

    for group in constraint_groups.iter() {
        // recompute adjustment degree only when it has changed
        if group.degree_adjustment != degree_adjustment {
            degree_adjustment = group.degree_adjustment;
            xp = x.exp(degree_adjustment.into());
        }
        // evaluate the group and save the result
        result.push(group.evaluate(state, ce_step, x, xp));
    }
}

// ASSERTION CONSTRAINT GROUP
// ================================================================================================

/// Contains constraints all having the same divisor. The constraints are separated into single
/// value constraints, small polynomial constraints, and large polynomial constraints.
pub struct AssertionConstraintGroup {
    degree_adjustment: u32,
    single_value_constraints: Vec<SingleValueConstraint>,
    small_poly_constraints: Vec<SmallPolyConstraint>,
    large_poly_constraints: Vec<LargePolyConstraint>,
}

impl AssertionConstraintGroup {
    /// Creates a new specialized constraint group; twiddles and ce_blowup_factor are passed in for
    /// evaluating large polynomial constraints (if any).
    pub fn new(
        group: &common::AssertionConstraintGroup,
        twiddles: &[BaseElement],
        ce_blowup_factor: usize,
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
                // evaluate the polynomial over the entire constraint evaluation domain
                let mut values = vec![BaseElement::ZERO; twiddles.len() * 2];
                values[..constraint.poly().len()].copy_from_slice(constraint.poly());
                fft::evaluate_poly(&mut values, twiddles);

                result.large_poly_constraints.push(LargePolyConstraint {
                    register: constraint.register(),
                    values,
                    step_offset: constraint.step_offset() * ce_blowup_factor,
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
        // separately track the degree-adjusted and un-adjusted results so that we can
        // perform only one multiplication by `xp` at the end.
        let mut result = E::ZERO;
        let mut result_adj = E::ZERO;

        // evaluate all single-value constraints
        for constraint in self.single_value_constraints.iter() {
            let (ev, ev_adj) = constraint.evaluate(state);
            result = result + ev;
            result_adj = result_adj + ev_adj;
        }

        // evaluate all small polynomial constraints
        for constraint in self.small_poly_constraints.iter() {
            let (ev, ev_adj) = constraint.evaluate(state, x);
            result = result + ev;
            result_adj = result_adj + ev_adj;
        }

        // evaluate all large polynomial constraints
        for constraint in self.large_poly_constraints.iter() {
            let (ev, ev_adj) = constraint.evaluate(state, ce_step);
            result = result + ev;
            result_adj = result_adj + ev_adj;
        }

        result + result_adj * xp
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
    pub fn evaluate<E: FieldElement + From<BaseElement>>(&self, state: &[E]) -> (E, E) {
        let evaluation = state[self.register] - E::from(self.value);
        (
            evaluation * E::from(self.coefficients.0),
            evaluation * E::from(self.coefficients.1),
        )
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
    pub fn evaluate<E: FieldElement + From<BaseElement>>(&self, state: &[E], x: E) -> (E, E) {
        let x = x * E::from(self.x_offset);
        let assertion_value = self
            .poly
            .iter()
            .rev()
            .fold(E::ZERO, |result, coeff| result * x + E::from(*coeff));
        let evaluation = state[self.register] - assertion_value;
        (
            evaluation * E::from(self.coefficients.0),
            evaluation * E::from(self.coefficients.1),
        )
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
    ) -> (E, E) {
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
        (
            evaluation * E::from(self.coefficients.0),
            evaluation * E::from(self.coefficients.1),
        )
    }
}
