use crate::{ComputationContext, ConstraintDivisor};
use math::{
    field::{BaseElement, FieldElement},
    polynom,
};

// ASSERTION CONSTRAINT GROUP
// ================================================================================================

/// A group of assertion constraints all having the same divisor.
#[derive(Debug, Clone)]
pub struct AssertionConstraintGroup {
    pub(super) constraints: Vec<AssertionConstraint>,
    pub(super) divisor: ConstraintDivisor,
    pub(super) degree_adjustment: u32,
}

impl AssertionConstraintGroup {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    pub fn new(context: &ComputationContext, divisor: ConstraintDivisor) -> Self {
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

// ASSERTION CONSTRAINT
// ================================================================================================

#[derive(Debug, Clone)]
pub struct AssertionConstraint {
    pub(super) register: usize,
    pub(super) poly: Vec<BaseElement>,
    pub(super) x_offset: BaseElement,
    pub(super) step_offset: usize,
    pub(super) cc: (BaseElement, BaseElement),
}

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

    /// Returns offset by which we need to multiply x before evaluating this constraint at x.
    pub fn x_offset(&self) -> BaseElement {
        self.x_offset
    }

    /// Returns offset by which we need to shift the domain before evaluating this constraint.
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
            // otherwise, we need to evaluate the polynomial at `x`; but first do the following:
            // 1. for assertions which don't fall on steps that are powers of two, we need to
            //    evaluate assertion polynomial at x * offset (instead of just x)
            // 2. map the coefficients of the polynomial into the evaluation field. If we are
            //    working in the base field, this has not effect; but if we are working in an
            //    extension field, coefficients of the polynomial are mapped from the base
            //    field into the extension field.
            let x = x * E::from(self.x_offset);
            polynom::eval(&self.poly, x)
        };
        // subtract assertion value from trace value
        trace_value - assertion_value
    }
}
