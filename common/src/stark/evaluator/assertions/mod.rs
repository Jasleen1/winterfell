use super::{ConstraintDivisor, ProofContext};

mod io_evaluator;
pub use io_evaluator::IoAssertionEvaluator;

// ASSERTION EVALUATOR TRAIT
// ================================================================================================

pub trait AssertionEvaluator {
    const MAX_CONSTRAINTS: usize;

    fn new(context: &ProofContext, assertions: &[Assertion], coefficients: &[u128]) -> Self;

    /// Evaluates assertion constraints at the specified `x` coordinate. The evaluations are
    /// saved into the `result` slice. This method is used by both the prover and the verifier.
    fn evaluate(&self, result: &mut [u128], state: &[u128], x: u128);

    /// Returns divisors for all assertion constraints.
    fn divisors(&self) -> &[ConstraintDivisor];
}

// ASSERTION STRUCT
// ================================================================================================

#[derive(Debug, Clone, Copy)]
pub struct Assertion {
    register: usize,
    step: usize,
    value: u128,
}

impl Assertion {
    pub fn new(register: usize, step: usize, value: u128) -> Assertion {
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

    pub fn value(&self) -> u128 {
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
