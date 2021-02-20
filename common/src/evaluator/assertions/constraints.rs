use super::Assertions;
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
    pub(crate) register: usize,
    pub(crate) poly: Vec<BaseElement>,
    pub(crate) x_offset: BaseElement,
    pub(crate) step_offset: usize,
    pub(crate) cc: (BaseElement, BaseElement),
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
    let assertions = assertions.into_vec();

    // compute inverse of the trace domain generator; this will be used for offset
    // computations when creating a new constraint
    let inv_g = context.generators().trace_domain.inv();

    // set up variables to track values from the previous iteration of the loop
    let mut stride = usize::MAX;
    let mut first_step = usize::MAX;
    let mut inv_twiddles = Vec::new();

    // iterate over all assertions, which are sorted first by stride and then by first_step
    // in ascending order
    for assertion in assertions {
        if assertion.stride != stride {
            // when strides change, we need to build new inv_twiddles and also
            // start a new assertion group
            stride = assertion.stride;
            first_step = assertion.first_step;

            // if an assertion consists of two values or more, we'll need to interpolate
            // an assertion polynomial from these values; for that, we'll need twiddles
            if assertion.num_values > 1 {
                inv_twiddles = build_inv_twiddles(assertion.num_values);
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

    // Returns degree of the largest constraint polynomial in this assertion group.
    pub fn max_poly_degree(&self) -> usize {
        let mut poly_size = 0;
        for constraint in self.constraints.iter() {
            if constraint.poly().len() > poly_size {
                poly_size = constraint.poly().len();
            }
        }
        poly_size - 1
    }

    pub fn add_constraint(&mut self, constraint: AssertionConstraint) {
        self.constraints.push(constraint);
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
        self.x_offset
    }

    pub fn step_offset(&self) -> usize {
        self.step_offset
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
            let x = x * E::from(self.x_offset);
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
