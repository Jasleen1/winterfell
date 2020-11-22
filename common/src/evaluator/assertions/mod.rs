use super::{ComputationContext, ConstraintDivisor, RandomGenerator};
use crate::errors::EvaluatorError;
use math::field::{BaseElement, FieldElement};

mod default_evaluator;
pub use default_evaluator::DefaultAssertionEvaluator;

// ASSERTION EVALUATOR TRAIT
// ================================================================================================

pub trait AssertionEvaluator {
    fn new(
        context: &ComputationContext,
        assertions: &[Assertion],
        coeff_prng: RandomGenerator,
    ) -> Result<Self, EvaluatorError>
    where
        Self: Sized;

    /// Evaluates assertion constraints at the specified `x` coordinate. The evaluations are
    /// saved into the `result` slice. This method is used by both the prover and the verifier.
    fn evaluate<E: FieldElement<PositiveInteger = u128> + From<BaseElement>>(
        &self,
        result: &mut [E],
        state: &[E],
        x: E,
    );

    /// Returns divisors for all assertion constraints.
    fn divisors(&self) -> &[ConstraintDivisor];
}

// ASSERTION
// ================================================================================================

#[derive(Debug, Clone, Copy)]
pub struct Assertion {
    register: usize,
    step: usize,
    value: BaseElement,
}

impl Assertion {
    pub fn new(register: usize, step: usize, value: BaseElement) -> Assertion {
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

    pub fn value(&self) -> BaseElement {
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
