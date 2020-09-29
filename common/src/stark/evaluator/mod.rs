use super::{ProofContext, PublicCoin};
use math::field;

mod transition;
pub use transition::{group_transition_constraints, TransitionEvaluator};

mod assertions;
pub use assertions::{Assertion, AssertionEvaluator, IoAssertionEvaluator};

mod constraints;
pub use constraints::{ConstraintDegree, ConstraintDivisor};

#[cfg(test)]
mod tests;

// CONSTRAINT EVALUATOR
// ================================================================================================

pub struct ConstraintEvaluator<T: TransitionEvaluator, A: AssertionEvaluator> {
    assertions: A,
    transition: T,
    context: ProofContext,
    transition_degree_map: Vec<(u128, Vec<usize>)>,
}

impl<T: TransitionEvaluator, A: AssertionEvaluator> ConstraintEvaluator<T, A> {
    pub fn new<C: PublicCoin>(
        coin: &C,
        context: &ProofContext,
        assertions: Vec<Assertion>,
    ) -> Self {
        assert!(
            !assertions.is_empty(),
            "at least one assertion must be provided"
        );

        // TODO: switch over to an iterator to generate coefficients
        let (t_coefficients, a_coefficients) = Self::build_coefficients(coin);
        let transition = T::new(context, &t_coefficients);
        let transition_degree_map = group_transition_constraints(
            context.composition_degree(),
            transition.degrees(),
            context.trace_length(),
        );

        let assertions = A::new(&context, &assertions, &a_coefficients);

        ConstraintEvaluator {
            transition,
            assertions,
            context: context.clone(),
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

        // TODO: save individual transition evaluations

        if step % self.context.ce_blowup_factor() == 0 {
            // TODO check for zeros
        }

        let t_evaluation = self.merge_transition_evaluations(&t_evaluations, x);

        // evaluate boundary constraints defined by assertions
        let (i_evaluation, f_evaluation) = self.assertions.evaluate(current, x);

        (t_evaluation, i_evaluation, f_evaluation)
    }

    pub fn evaluate_at(&self, current: &[u128], next: &[u128], x: u128) -> (u128, u128, u128) {
        // evaluate transition constraints and merge them into a single value
        let t_evaluations = self.transition.evaluate_at(current, next, x);
        let t_evaluation = self.merge_transition_evaluations(&t_evaluations, x);

        // evaluate boundary constraints defined by assertions
        let (i_evaluation, f_evaluation) = self.assertions.evaluate(current, x);

        (t_evaluation, i_evaluation, f_evaluation)
    }

    pub fn constraint_divisors(&self) -> Vec<ConstraintDivisor> {
        // TODO: build and save constraint divisors at construction time?
        let x_at_last_step = self.get_x_at_last_step();
        vec![
            ConstraintDivisor::from_transition(self.trace_length(), x_at_last_step),
            ConstraintDivisor::from_assertion(1),
            ConstraintDivisor::from_assertion(x_at_last_step),
        ]
    }

    /// Returns size of the constraint evaluation domain.
    pub fn ce_domain_size(&self) -> usize {
        self.context.ce_domain_size()
    }

    /// Returns size of low-degree extension domain.
    pub fn lde_domain_size(&self) -> usize {
        self.context.lde_domain_size()
    }

    pub fn trace_length(&self) -> usize {
        self.context.trace_length()
    }

    pub fn blowup_factor(&self) -> usize {
        self.context.options().blowup_factor()
    }

    pub fn get_x_at_last_step(&self) -> u128 {
        self.get_x_at(self.trace_length() - 1)
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

    fn build_coefficients<C: PublicCoin>(coin: &C) -> (Vec<u128>, Vec<u128>) {
        let num_t_coefficients = T::MAX_CONSTRAINTS * 2;
        let num_a_coefficients = A::MAX_CONSTRAINTS * 2;

        let coefficients =
            coin.draw_constraint_coefficients(num_t_coefficients + num_a_coefficients);
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
