use math::field;

mod transition;
pub use transition::{group_transition_constraints, TransitionEvaluator};

mod assertions;
pub use assertions::{Assertion, AssertionEvaluator, IoAssertionEvaluator};

#[cfg(test)]
pub use transition::tests::FibEvaluator;

#[cfg(test)]
mod tests;

// CONSTRAINT EVALUATOR
// ================================================================================================

pub struct ConstraintEvaluator<T: TransitionEvaluator, A: AssertionEvaluator> {
    assertions: A,
    transition: T,
    trace_info: TraceInfo,
    max_constraint_degree: usize,
    transition_degree_map: Vec<(u128, Vec<usize>)>,
}

impl<T: TransitionEvaluator, A: AssertionEvaluator> ConstraintEvaluator<T, A> {
    pub fn new(seed: [u8; 32], trace_info: &TraceInfo, assertions: &Vec<Assertion>) -> Self {
        assert!(
            assertions.len() > 0,
            "at least one assertion must be provided"
        );

        // TODO: switch over to an iterator to generate coefficients
        let (t_coefficients, a_coefficients) = Self::build_coefficients(seed);
        let transition = T::new(trace_info, &t_coefficients);
        let max_constraint_degree = *transition.degrees().iter().max().unwrap();
        let transition_degree_map =
            group_transition_constraints(transition.degrees(), trace_info.length());

        let composition_degree = get_composition_degree(trace_info.length(), max_constraint_degree);
        let assertions = A::new(assertions, trace_info, composition_degree, &a_coefficients);

        ConstraintEvaluator {
            transition,
            assertions,
            trace_info: *trace_info,
            max_constraint_degree,
            transition_degree_map,
        }
    }

    pub fn evaluate(
        &self,
        current: &[u128],
        next: &[u128],
        x: u128,
        step: usize,
    ) -> (u128, u128, u128) {
        // evaluate transition constraints and merge them into a single value
        let t_evaluations = self.transition.evaluate(current, next, step);
        let t_evaluation = self.merge_transition_evaluations(&t_evaluations, x);

        // evaluate boundary constraints defined by assertions
        let (i_evaluation, f_evaluation) = self.assertions.evaluate(current, x);

        (t_evaluation, i_evaluation, f_evaluation)
    }

    pub fn constraint_divisors(&self) -> Vec<ConstraintDivisor> {
        // TODO: build and save constraint divisors at construction time?
        let x_at_last_step = self.get_x_at(self.trace_length() - 1);
        vec![
            ConstraintDivisor::from_transition(self.trace_length(), x_at_last_step),
            ConstraintDivisor::from_assertion(1),
            ConstraintDivisor::from_assertion(x_at_last_step),
        ]
    }

    /// Returns size of the constraint evaluation domain.
    pub fn ce_domain_size(&self) -> usize {
        // domain for constraint evaluation must be at least a multiple of
        // max constraint degree; but we also put a floor at 2x so that constraint
        // composition can work correctly
        let ce_blowup_factor = std::cmp::max(self.max_constraint_degree, 2);
        self.trace_length() * ce_blowup_factor
    }

    /// Returns size of low-degree extension domain.
    pub fn lde_domain_size(&self) -> usize {
        self.trace_info.lde_domain_size()
    }

    pub fn max_constraint_degree(&self) -> usize {
        self.max_constraint_degree
    }

    pub fn deep_composition_degree(&self) -> usize {
        get_composition_degree(self.trace_length(), self.max_constraint_degree()) - 1
    }

    pub fn trace_length(&self) -> usize {
        self.trace_info.length()
    }

    pub fn blowup_factor(&self) -> usize {
        self.trace_info.blowup()
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    fn merge_transition_evaluations(&self, evaluations: &[u128], x: u128) -> u128 {
        let cc = self.transition.composition_coefficients();

        // there must be two coefficients for each constraint evaluation
        debug_assert_eq!(evaluations.len() * 2, cc.len());

        let mut result = field::ZERO;

        let mut i = 0;
        for (incremental_degree, constraints) in self.transition_degree_map.iter() {
            // for each group of constraints with the same degree, separately compute
            // combinations of D(x) and D(x) * x^p
            let mut result_adj = field::ZERO;
            for &constraint_idx in constraints.iter() {
                let evaluation = evaluations[constraint_idx];
                result = field::add(result, field::mul(evaluation, cc[i * 2]));
                result_adj = field::add(result_adj, field::mul(evaluation, cc[i * 2 + 1]));
                i += 1;
            }

            // increase the degree of D(x) * x^p
            let xp = field::exp(x, *incremental_degree);
            result = field::add(result, field::mul(result_adj, xp));
        }

        result
    }

    fn build_coefficients(seed: [u8; 32]) -> (Vec<u128>, Vec<u128>) {
        let num_t_coefficients = T::MAX_CONSTRAINTS * 2;
        let num_a_coefficients = A::MAX_CONSTRAINTS * 2;

        let coefficients = field::prng_vector(seed, num_t_coefficients + num_a_coefficients);
        (
            coefficients[..num_t_coefficients].to_vec(),
            coefficients[num_t_coefficients..].to_vec(),
        )
    }

    // Returns x in the trace domain at the specified step
    fn get_x_at(&self, step: usize) -> u128 {
        let trace_root = field::get_root_of_unity(self.trace_length());
        field::exp(trace_root, step as u128)
    }
}

// TRACE INFO
// ================================================================================================

#[derive(Copy, Clone)]
pub struct TraceInfo(usize, usize, usize);

impl TraceInfo {
    pub fn new(width: usize, length: usize, blowup: usize) -> Self {
        TraceInfo(width, length, blowup)
    }

    pub fn width(&self) -> usize {
        self.0
    }

    pub fn length(&self) -> usize {
        self.1
    }

    pub fn blowup(&self) -> usize {
        self.2
    }

    pub fn lde_domain_size(&self) -> usize {
        self.length() * self.blowup()
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

// HELPER FUNCTIONS
// ================================================================================================

// TODO: provide explanation
fn get_composition_degree(trace_length: usize, max_constraint_degree: usize) -> usize {
    std::cmp::max(max_constraint_degree - 1, 1) * trace_length
}
