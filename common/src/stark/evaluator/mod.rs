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

    #[cfg(debug_assertions)]
    pub t_evaluations: Vec<Vec<u128>>,
}

impl<T: TransitionEvaluator, A: AssertionEvaluator> ConstraintEvaluator<T, A> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

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
            #[cfg(debug_assertions)]
            t_evaluations: transition.degrees().iter().map(|_| Vec::new()).collect(),

            transition,
            assertions,
            context: context.clone(),
            transition_degree_map,
        }
    }

    // EVALUATION METHODS
    // --------------------------------------------------------------------------------------------

    /// TODO: add comments
    pub fn evaluate_at_step(
        &mut self,
        current: &[u128],
        next: &[u128],
        x: u128,
        step: usize,
    ) -> (u128, u128, u128) {
        // evaluate transition constraints
        let t_evaluations = self.transition.evaluate(current, next, step);

        // when in debug mode, save transition constraint evaluations before they are merged
        // so that we can check their degrees later
        #[cfg(debug_assertions)]
        self.save_transition_evaluations(&t_evaluations);

        // if the constraints should evaluate to all zeros at this step, make sure they do;
        // then, merge the constraints into a single value; we can do this here because all
        // transition constraints have the same denominator
        let t_evaluation = if self.should_evaluate_to_zero_at(step) {
            let step = step / self.ce_blowup_factor();
            for &evaluation in t_evaluations.iter() {
                assert!(
                    evaluation == field::ZERO,
                    "transition constraint at step {} were not satisfied",
                    step
                );
            }
            // if all transition constraint evaluations are zeros, the combination is also zero
            field::ZERO
        } else {
            self.merge_transition_evaluations(&t_evaluations, x)
        };

        // evaluate boundary constraints defined by assertions
        let (i_evaluation, f_evaluation) = self.assertions.evaluate(current, x);

        (t_evaluation, i_evaluation, f_evaluation)
    }

    /// TODO: add comments
    pub fn evaluate_at_x(&self, current: &[u128], next: &[u128], x: u128) -> (u128, u128, u128) {
        // evaluate transition constraints and merge them into a single value
        let t_evaluations = self.transition.evaluate_at(current, next, x);
        let t_evaluation = self.merge_transition_evaluations(&t_evaluations, x);

        // evaluate boundary constraints defined by assertions
        let (i_evaluation, f_evaluation) = self.assertions.evaluate(current, x);

        (t_evaluation, i_evaluation, f_evaluation)
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    pub fn trace_length(&self) -> usize {
        self.context.trace_length()
    }

    /// Returns size of the constraint evaluation domain.
    pub fn ce_domain_size(&self) -> usize {
        self.context.ce_domain_size()
    }

    pub fn ce_blowup_factor(&self) -> usize {
        self.context.ce_blowup_factor()
    }

    /// Returns size of low-degree extension domain.
    pub fn lde_domain_size(&self) -> usize {
        self.context.lde_domain_size()
    }

    pub fn lde_blowup_factor(&self) -> usize {
        self.context.options().blowup_factor()
    }

    pub fn constraint_divisors(&self) -> Vec<ConstraintDivisor> {
        // TODO: build and save constraint divisors at construction time?
        let x_at_last_step = get_x_at_last_step(self.context.trace_length());
        vec![
            ConstraintDivisor::from_transition(self.context.trace_length(), x_at_last_step),
            ConstraintDivisor::from_assertion(1),
            ConstraintDivisor::from_assertion(x_at_last_step),
        ]
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    #[inline(always)]
    fn should_evaluate_to_zero_at(&self, step: usize) -> bool {
        (step & (self.ce_blowup_factor() - 1) == 0) // same as: step % ce_blowup_factor == 0
        && (step != self.ce_domain_size() - self.ce_blowup_factor())
    }

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

    // DEBUG HELPERS
    // --------------------------------------------------------------------------------------------

    #[cfg(debug_assertions)]
    fn save_transition_evaluations(&mut self, evaluations: &[u128]) {
        for (i, constraint) in self.t_evaluations.iter_mut().enumerate() {
            constraint.push(evaluations[i]);
        }
    }

    #[cfg(debug_assertions)]
    pub fn validate_transition_degrees(&mut self) {
        use math::{fft, polynom};

        // collect expected degrees for all transition constraints
        let expected_degrees: Vec<_> = self
            .transition
            .degrees()
            .iter()
            .map(|d| d.get_evaluation_degree(self.trace_length()))
            .collect();

        // collect actual degrees for all transition constraints by interpolating saved
        // constraint evaluations into polynomials and checking their degree; also
        // determine max transition constraint degree
        let mut actual_degrees = Vec::with_capacity(expected_degrees.len());
        let mut max_degree = 0;
        let inv_twiddles = fft::get_inv_twiddles(
            self.context.generators().ce_domain,
            self.context.ce_domain_size(),
        );
        for evaluations in self.t_evaluations.iter() {
            let mut poly = evaluations.clone();
            fft::interpolate_poly(&mut poly, &inv_twiddles, true);
            let degree = polynom::degree_of(&poly);
            actual_degrees.push(degree);

            max_degree = std::cmp::max(max_degree, degree);
        }

        // make sure expected and actual degrees are equal
        if expected_degrees != actual_degrees {
            panic!(
                "transition constraint degrees didn't match\nexpected: {:>3?}\nactual:   {:>3?}",
                expected_degrees, actual_degrees
            );
        }

        // make sure evaluation domain size does not exceed the size required by max degree
        let expected_domain_size =
            std::cmp::max(max_degree, self.trace_length() + 1).next_power_of_two();
        if expected_domain_size != self.ce_domain_size() {
            panic!(
                "incorrect constraint evaluation domain size; expected {}, actual: {}",
                expected_domain_size,
                self.ce_domain_size()
            );
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn get_x_at_last_step(trace_length: usize) -> u128 {
    let trace_root = field::get_root_of_unity(trace_length);
    let last_step = (trace_length - 1) as u128;
    field::exp(trace_root, last_step)
}
