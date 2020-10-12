use math::field::{f128::FieldElement, StarkField};

// CONSTRAINT DEGREE
// ================================================================================================

/// Describes constraint degree as a combination of multiplications of non-cyclic and cyclic
/// register multiplications. For example, degree of a constraint which requires multiplication
/// of two regular registers, and a register with cycle 32 can be represented as:
///   base: 2
///   cycles: [32]
#[derive(Clone)]
pub struct ConstraintDegree {
    base: usize,
    cycles: Vec<usize>,
}

impl ConstraintDegree {
    pub fn new(base: usize) -> Self {
        ConstraintDegree {
            base,
            cycles: vec![],
        }
    }

    pub fn with_cycles(base: usize, cycles: Vec<usize>) -> Self {
        ConstraintDegree { base, cycles }
    }

    pub fn get_evaluation_degree(&self, trace_length: usize) -> usize {
        let mut result = self.base * (trace_length - 1);
        for cycle_length in self.cycles.iter() {
            result += (trace_length / cycle_length) * (cycle_length - 1);
        }
        result
    }

    /// Returns a minimum blowup factor needed to evaluate constraint of this degree.
    pub fn min_blowup_factor(&self) -> usize {
        (self.base + self.cycles.len()).next_power_of_two()
    }
}

// CONSTRAINT DIVISOR
// ================================================================================================

/// Describes constraint divisor as a combination of a sparse polynomial and exclusion points.
/// For example (x^a - 1) / (x - b) can be represented as:
///   numerator: vec![(a, 1)]
///   exclude: vec![b]
#[derive(Clone)]
pub struct ConstraintDivisor {
    numerator: Vec<(usize, FieldElement)>,
    exclude: Vec<FieldElement>,
}

impl ConstraintDivisor {
    /// Builds divisor for transition constraints
    pub fn from_transition(trace_length: usize, x_at_last_step: FieldElement) -> Self {
        ConstraintDivisor {
            numerator: vec![(trace_length, FieldElement::ONE)],
            exclude: vec![x_at_last_step],
        }
    }

    /// Builds divisor for assertion constraint
    pub fn from_assertion(x: FieldElement) -> Self {
        ConstraintDivisor {
            numerator: vec![(1, x)],
            exclude: vec![],
        }
    }

    pub fn numerator(&self) -> &[(usize, FieldElement)] {
        &self.numerator
    }

    pub fn exclude(&self) -> &[FieldElement] {
        &self.exclude
    }

    /// Returns the degree of the divisor polynomial
    pub fn degree(&self) -> usize {
        let numerator_degree = self.numerator[0].0;
        let denominator_degree = self.exclude.len();
        numerator_degree - denominator_degree
    }

    pub fn evaluate_at(&self, x: FieldElement) -> FieldElement {
        let mut result = FieldElement::ONE;

        for (degree, constant) in self.numerator.iter() {
            let v = FieldElement::exp(x, *degree as u128);
            let v = v - *constant;
            result = result * v;
        }

        for exception in self.exclude.iter() {
            let v = x - *exception;
            result = result / v;
        }

        result
    }
}
