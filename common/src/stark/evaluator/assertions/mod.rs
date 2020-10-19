use super::{ConstraintDivisor, ProofContext};
use math::field::{FieldElement, StarkField};

mod io_evaluator;
pub use io_evaluator::IoAssertionEvaluator;

mod basic_evaluator;
pub use basic_evaluator::BasicAssertionEvaluator;

// ASSERTION EVALUATOR TRAIT
// ================================================================================================

pub trait AssertionEvaluator {
    const MAX_CONSTRAINTS: usize;

    fn new(context: &ProofContext, assertions: &[Assertion], coefficients: &[FieldElement])
        -> Self;

    /// Evaluates assertion constraints at the specified `x` coordinate. The evaluations are
    /// saved into the `result` slice. This method is used by both the prover and the verifier.
    fn evaluate(&self, result: &mut [FieldElement], state: &[FieldElement], x: FieldElement);

    /// Returns divisors for all assertion constraints.
    fn divisors(&self) -> &[ConstraintDivisor];
}

// ASSERTION
// ================================================================================================

#[derive(Debug, Clone, Copy)]
pub struct Assertion {
    register: usize,
    step: usize,
    value: FieldElement,
}

impl Assertion {
    pub fn new(register: usize, step: usize, value: FieldElement) -> Assertion {
        Assertion {
            register,
            step,
            value,
        }
    }

    pub fn register(&self) -> usize {
        self.register
    }

    pub fn step(&self) -> usize {
        self.step
    }

    pub fn value(&self) -> FieldElement {
        self.value
    }
}

impl std::fmt::Display for Assertion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "trace({}, {}) == {}",
            self.register, self.step, self.value
        )
    }
}

// ASSERTION CONSTRAINT
// ================================================================================================

#[derive(Debug, Clone)]
struct AssertionConstraint {
    register: usize,
    value: FieldElement,
}

// ASSERTION CONSTRAINT GROUP
// ================================================================================================

/// A group of assertion constraints all having the same divisor.
#[derive(Debug, Clone)]
struct AssertionConstraintGroup {
    constraints: Vec<AssertionConstraint>,
    coefficients: Vec<(FieldElement, FieldElement)>,
    divisor: ConstraintDivisor,
    degree_adjustment: u128,
}

impl AssertionConstraintGroup {
    fn new(context: &ProofContext, divisor: ConstraintDivisor) -> Self {
        // We want to make sure that once we divide a constraint polynomial by its divisor, the
        // degree of the resulting polynomials will be exactly equal to the composition_degree.
        // Assertion constraint degree is always deg(trace). So, the adjustment degree is simply:
        // deg(composition) + deg(divisor) - deg(trace)
        let target_degree = context.composition_degree() + divisor.degree();
        let degree_adjustment = (target_degree - context.trace_poly_degree()) as u128;

        AssertionConstraintGroup {
            constraints: Vec::new(),
            coefficients: Vec::new(),
            divisor,
            degree_adjustment,
        }
    }

    fn evaluate(&self, state: &[FieldElement], xp: FieldElement) -> FieldElement {
        let mut result = FieldElement::ZERO;
        let mut result_adj = FieldElement::ZERO;

        for (constraint, coefficients) in self.constraints.iter().zip(self.coefficients.iter()) {
            let value = state[constraint.register] - constraint.value;
            result = result + value * coefficients.0;
            result_adj = result_adj + value * coefficients.1;
        }

        result + result_adj * xp
    }
}
