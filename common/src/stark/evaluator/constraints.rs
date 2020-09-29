// CONSTRAINT DEGREE
// ================================================================================================

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
        let mut result = self.base * trace_length;
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
pub struct ConstraintDivisor {
    numerator: Vec<(usize, u128)>,
    exclude: Vec<u128>,
}

impl ConstraintDivisor {
    /// Builds divisor for transition constraints
    pub fn from_transition(trace_length: usize, x_at_last_step: u128) -> Self {
        ConstraintDivisor {
            numerator: vec![(trace_length, 1)],
            exclude: vec![x_at_last_step],
        }
    }

    /// Builds divisor for assertion constraint
    pub fn from_assertion(value: u128) -> Self {
        ConstraintDivisor {
            numerator: vec![(1, value)],
            exclude: vec![],
        }
    }

    pub fn numerator(&self) -> &[(usize, u128)] {
        &self.numerator
    }

    pub fn exclude(&self) -> &[u128] {
        &self.exclude
    }

    /// Returns the degree of the divisor polynomial
    pub fn degree(&self) -> usize {
        let numerator_degree = self.numerator[0].0;
        let denominator_degree = self.exclude.len();
        numerator_degree - denominator_degree
    }
}
