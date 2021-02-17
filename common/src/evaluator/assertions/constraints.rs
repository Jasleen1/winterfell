use super::{Assertions, CyclicAssertion, PointAssertion};
use crate::{ComputationContext, ConstraintDivisor};
use crypto::RandomElementGenerator;
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
};

// TYPES AND INTERFACES
// ================================================================================================

/// A group of assertion constraints all having the same divisor.
#[derive(Debug, Clone)]
pub struct AssertionConstraintGroup {
    constraints: Vec<AssertionConstraint>,
    divisor: ConstraintDivisor,
    degree_adjustment: u32,
}

#[derive(Debug, Clone)]
pub struct AssertionConstraint {
    register: usize,
    poly: Vec<BaseElement>,
    offset: BaseElement,
    cc: (BaseElement, BaseElement),
}

// CONSTRAINT BUILDER
// ================================================================================================

pub fn build_assertion_constraints(
    context: &ComputationContext,
    assertions: Assertions,
    mut coeff_prng: RandomElementGenerator,
) -> Vec<AssertionConstraintGroup> {
    // group assertions by step - i.e.: assertions for the first step are grouped together,
    // assertions for the last step are grouped together etc.
    let mut groups: Vec<AssertionConstraintGroup> = Vec::new();

    // break the assertion collection into lists of individual assertions
    let (point_assertions, cyclic_assertions) = assertions.into_lists();

    // build constraints for point assertions
    if !point_assertions.is_empty() {
        // this will store step values from the previous iteration of the loop
        let mut step = usize::MAX;

        // iterate over all point assertions which are sorted first by step, and then by register
        // in ascending order
        for assertion in point_assertions {
            if step != assertion.step {
                // step changes, create a new assertion group; this results in point assertions
                // which are made against the same step to be grouped together
                step = assertion.step;
                groups.push(AssertionConstraintGroup::for_point_assertions(
                    context, step,
                ));
            }

            // add a new assertion constraint to current group (last group in the list)
            groups
                .last_mut()
                .unwrap()
                .add_point_assertion(assertion, &mut coeff_prng);
        }
    }

    // build constraints for cyclic assertions
    if !cyclic_assertions.is_empty() {
        // compute inverse of the trace domain generator; this will be used for
        // offset computations when creating a new constraint
        let inv_g = context.generators().trace_domain.inv();

        // set up variables to track values from the previous iteration of the loop
        let mut stride = usize::MAX;
        let mut first_step = usize::MAX;
        let mut inv_twiddles = Vec::new();

        // iterate over all cyclic assertions, which are sorted first by stride and then
        // by first_step in ascending order
        for assertion in cyclic_assertions {
            if assertion.stride != stride {
                // when strides change, we need to build new inv_twiddles and also
                // start a new assertion group
                stride = assertion.stride;
                first_step = assertion.first_step;

                // TODO: avoid building twiddles when not needed
                let num_asserted_values = context.trace_length() / stride;
                if num_asserted_values > 1 {
                    inv_twiddles = build_inv_twiddles(num_asserted_values);
                }
                groups.push(AssertionConstraintGroup::for_cyclic_assertions(
                    context, first_step, stride,
                ));
            } else if assertion.first_step != first_step {
                // if only the first_step changed, we can use inv_twiddles from the
                // previous iteration, but we do need to start a new assertion group
                first_step = assertion.first_step;
                groups.push(AssertionConstraintGroup::for_cyclic_assertions(
                    context, first_step, stride,
                ));
            }

            // add a new assertion constraint to the current group (last group in the list)
            groups.last_mut().unwrap().add_cyclic_assertion(
                assertion,
                inv_g,
                &inv_twiddles,
                &mut coeff_prng,
            );
        }
    }

    // make sure groups are sorted by adjustment degree
    groups.sort_by_key(|c| c.degree_adjustment);

    groups
}

// ASSERTION CONSTRAINT GROUP IMPLEMENTATION
// ================================================================================================

impl AssertionConstraintGroup {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    fn new(context: &ComputationContext, divisor: ConstraintDivisor) -> Self {
        // We want to make sure that once we divide a constraint polynomial by its divisor, the
        // degree of the resulting polynomials will be exactly equal to the composition_degree.
        // Assertion constraint degree is always deg(trace). So, the adjustment degree is simply:
        // deg(composition) + deg(divisor) - deg(trace)
        let target_degree = context.composition_degree() + divisor.degree();
        let degree_adjustment = (target_degree - context.trace_poly_degree()) as u32;

        AssertionConstraintGroup {
            constraints: Vec::new(),
            divisor,
            degree_adjustment,
        }
    }

    /// Returns a new assertion group which can hold point assertions at the specified step.
    pub fn for_point_assertions(context: &ComputationContext, step: usize) -> Self {
        let divisor = ConstraintDivisor::from_point_assertion(step, context);
        AssertionConstraintGroup::new(context, divisor)
    }

    /// Returns a new assertion group which can hold cyclic assertions for the specified
    /// first_step and stride.
    pub fn for_cyclic_assertions(
        context: &ComputationContext,
        first_step: usize,
        stride: usize,
    ) -> Self {
        let divisor = ConstraintDivisor::from_cyclic_assertion(first_step, stride, context);
        Self::new(context, divisor)
    }

    // CONSTRAINT ADDERS
    // --------------------------------------------------------------------------------------------
    pub fn add_point_assertion(
        &mut self,
        assertion: PointAssertion,
        coeff_prng: &mut RandomElementGenerator,
    ) {
        self.constraints.push(AssertionConstraint {
            register: assertion.register,
            poly: vec![assertion.value],
            offset: BaseElement::ZERO,
            cc: coeff_prng.draw_pair(),
        });
    }

    pub fn add_cyclic_assertion(
        &mut self,
        assertion: CyclicAssertion,
        inv_g: BaseElement,
        inv_twiddles: &[BaseElement],
        coeff_prng: &mut RandomElementGenerator,
    ) {
        // build a polynomial which evaluates to constraint values at asserted steps; for
        // single-value assertions we use the value as constant coefficient of degree 0
        // polynomial; but if there is more than value, we need to interpolate them into
        // a polynomial using inverse FFT
        let mut offset = BaseElement::ONE;
        let mut poly = assertion.values;
        if poly.len() > 1 {
            fft::interpolate_poly(&mut poly, &inv_twiddles, true);
            if assertion.first_step != 0 {
                // if the assertions don't fall on the steps which are powers of two, we can't
                // use FFT to interpolate the values into a polynomial. This would make such
                // assertions quite impractical. To get around this, we still use FFT to build
                // the polynomial, but then we evaluate it as f(x * offset) instead of f(x)
                offset = inv_g.exp((assertion.first_step as u64).into());
            }
        }

        self.constraints.push(AssertionConstraint {
            register: assertion.register,
            poly,
            offset,
            cc: coeff_prng.draw_pair(),
        });
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a list of constraints for this assertion group.
    pub fn constraints(&self) -> &[AssertionConstraint] {
        &self.constraints
    }

    /// Returns a divisor applicable to all constraints in this assertion group.
    pub fn divisor(&self) -> &ConstraintDivisor {
        &self.divisor
    }

    /// Returns a degree adjustment factor for all constraints in this assertion group.
    pub fn degree_adjustment(&self) -> u32 {
        self.degree_adjustment
    }
}

// ASSERTION CONSTRAINT IMPLEMENTATION
// ================================================================================================

impl AssertionConstraint {
    /// Returns index of the register against which this constraint applies.
    pub fn register(&self) -> usize {
        self.register
    }

    /// Returns constraint polynomial for this constraint.
    pub fn poly(&self) -> &[BaseElement] {
        &self.poly
    }

    /// Returns composition coefficients for this constraint.
    pub fn cc(&self) -> &(BaseElement, BaseElement) {
        &self.cc
    }

    pub fn x_offset(&self) -> BaseElement {
        self.offset
    }

    /// Evaluates this constraint at the specified point `x` by computing trace_value - P(x).
    /// trace_value is assumed to be evaluation of a trace polynomial at `x`.
    pub fn evaluate_at<E: FieldElement + From<BaseElement>>(&self, x: E, trace_value: E) -> E {
        let assertion_value = if self.poly.len() == 1 {
            // if constraint polynomial consists of just a constant, use that constant
            E::from(self.poly[0])
        } else {
            // otherwise, we need to evaluate the polynomial at `x` (using Horner method); but
            // first we need to do the following:
            // 1. for assertions which don't fall on steps that are powers of two, we need to
            //    evaluate assertion polynomial at x * offset (instead of just x)
            // 2. map the coefficients of the polynomial into the evaluation field. If we are
            //    working in the base field, this has not effect; but if we are working in an
            //    extension field, coefficients of the polynomial are mapped from the base
            //    field into the extension field.
            let x = x * E::from(self.offset);
            self.poly
                .iter()
                .rev()
                .fold(E::ZERO, |result, coeff| result * x + E::from(*coeff))
        };
        // subtract assertion value from trace value; when assertion is valid, this should be 0
        trace_value - assertion_value
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_inv_twiddles(num_assertion_values: usize) -> Vec<BaseElement> {
    let g = BaseElement::get_root_of_unity(num_assertion_values.trailing_zeros());
    fft::get_inv_twiddles(g, num_assertion_values)
}
