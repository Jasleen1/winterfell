use super::{ProofContext, PublicCoin};
use math::field::{FieldElement, StarkField};

mod transition;
pub use transition::{group_transition_constraints, TransitionEvaluator};

mod assertions;
pub use assertions::{Assertion, AssertionEvaluator, BasicAssertionEvaluator};

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
    evaluations: Vec<FieldElement>,
    t_evaluations: Vec<FieldElement>,
    divisors: Vec<ConstraintDivisor>,
    transition_degree_map: Vec<(u128, Vec<usize>)>,

    #[cfg(debug_assertions)]
    t_evaluation_table: Vec<Vec<FieldElement>>,
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

        // build coefficients for constraint combination
        // TODO: switch over to an iterator to generate coefficients?
        let (t_coefficients, a_coefficients) = Self::build_coefficients(coin);

        // instantiate transition constraint evaluator and group constraints by their
        // degree for more efficient combination later
        // TODO: move constraint grouping into transition constraint evaluator?
        let transition = T::new(context, &t_coefficients);
        let transition_degree_map = group_transition_constraints(
            context.composition_degree(),
            transition.degrees(),
            context.trace_length(),
        );

        // instantiate assertion constraint evaluator
        let assertions = A::new(&context, &assertions, &a_coefficients);

        // determine divisors for all constraints; since divisor for all transition constraints
        // are the same: (x^steps - 1) / (x - x_at_last_step), all transition constraints will be
        // merged into a single value, and the divisor for that value will be first in the list
        let mut divisors = vec![ConstraintDivisor::from_transition(
            context.trace_length(),
            context.get_trace_x_at(context.trace_length() - 1),
        )];
        divisors.extend_from_slice(assertions.divisors());

        ConstraintEvaluator {
            // in debug mode, we keep track of all evaluated transition constraints so that
            // we can verify that their stated degrees match their actual degrees
            #[cfg(debug_assertions)]
            t_evaluation_table: transition.degrees().iter().map(|_| Vec::new()).collect(),

            t_evaluations: vec![FieldElement::ZERO; transition.degrees().len()],
            transition,
            assertions,
            context: context.clone(),
            evaluations: vec![FieldElement::ZERO; divisors.len()],
            divisors,
            transition_degree_map,
        }
    }

    // EVALUATION METHODS
    // --------------------------------------------------------------------------------------------

    /// Evaluates transition and assertion constraints at the specified step in the evaluation
    /// domain. This method is used exclusively by the prover because some types of constraints
    /// can be evaluated much more efficiently when the step is known.
    pub fn evaluate_at_step(
        &mut self,
        current: &[FieldElement],
        next: &[FieldElement],
        x: FieldElement,
        step: usize,
    ) -> &[FieldElement] {
        // reset transition evaluation buffer
        self.t_evaluations
            .iter_mut()
            .for_each(|v| *v = FieldElement::ZERO);

        // evaluate transition constraints and save the results into t_evaluations buffer
        self.transition
            .evaluate_at_step(&mut self.t_evaluations, current, next, step);

        // when in debug mode, save transition constraint evaluations before merging them
        // so that we can check their degrees later
        #[cfg(debug_assertions)]
        for (i, column) in self.t_evaluation_table.iter_mut().enumerate() {
            column.push(self.t_evaluations[i]);
        }

        // merge transition constraint evaluations into a single value, and save this value
        // into the first slot of the evaluation buffer. we can do this here because all
        // transition constraints have the same divisor.
        // also: if the constraints should evaluate to all zeros at this step (which should
        // happen on steps which are multiples of ce_blowup_factor), make sure they do
        self.evaluations[0] = if self.should_evaluate_to_zero_at(step) {
            let step = step / self.ce_blowup_factor();
            for &evaluation in self.t_evaluations.iter() {
                assert!(
                    evaluation == FieldElement::ZERO,
                    "transition constraint at step {} were not satisfied",
                    step
                );
            }
            // if all transition constraint evaluations are zeros, the combination is also zero
            FieldElement::ZERO
        } else {
            // TODO: move this into transition constraint evaluator?
            self.merge_transition_evaluations(&self.t_evaluations, x)
        };

        // evaluate boundary constraints defined by assertions and save the result into
        // the evaluations buffer starting at slot 1
        self.assertions
            .evaluate(&mut self.evaluations[1..], current, x);

        &self.evaluations
    }

    /// Evaluates transition and assertion constraints at the specified x coordinate. This
    /// method is used to evaluate constraints at an out-of-domain point. At such a point
    /// there is no `step`, and so the above method cannot be used.
    pub fn evaluate_at_x(
        &mut self,
        current: &[FieldElement],
        next: &[FieldElement],
        x: FieldElement,
    ) -> &[FieldElement] {
        // reset transition evaluation buffer
        self.t_evaluations
            .iter_mut()
            .for_each(|v| *v = FieldElement::ZERO);

        // evaluate transition constraints and merge them into a single value
        self.transition
            .evaluate_at_x(&mut self.t_evaluations, current, next, x);
        self.evaluations[0] = self.merge_transition_evaluations(&self.t_evaluations, x);

        // evaluate boundary constraints defined by assertions and save the result into
        // the evaluations buffer starting at slot 1
        self.assertions
            .evaluate(&mut self.evaluations[1..], current, x);

        &self.evaluations
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

    pub fn constraint_divisors(&self) -> &[ConstraintDivisor] {
        &self.divisors
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    #[inline(always)]
    fn should_evaluate_to_zero_at(&self, step: usize) -> bool {
        (step & (self.ce_blowup_factor() - 1) == 0) // same as: step % ce_blowup_factor == 0
        && (step != self.ce_domain_size() - self.ce_blowup_factor())
    }

    /// Merges all transition constraint evaluations into a single value; we can do this
    /// because all transition constraint evaluations have the same divisor, and this
    /// divisor will be applied later to this single value.
    /// TODO: move into transition constraint evaluator?
    fn merge_transition_evaluations(
        &self,
        evaluations: &[FieldElement],
        x: FieldElement,
    ) -> FieldElement {
        let cc = self.transition.composition_coefficients();

        // there must be two coefficients for each constraint evaluation
        debug_assert_eq!(evaluations.len() * 2, cc.len());

        let mut result = FieldElement::ZERO;

        let mut i = 0;
        for (incremental_degree, constraints) in self.transition_degree_map.iter() {
            // for each group of constraints with the same degree, separately compute
            // combinations of D(x) and D(x) * x^p
            let mut result_adj = FieldElement::ZERO;
            for &constraint_idx in constraints.iter() {
                let evaluation = evaluations[constraint_idx];
                result = result + evaluation * cc[i * 2];
                result_adj = result_adj + evaluation * cc[i * 2 + 1];
                i += 1;
            }

            // increase the degree of D(x) * x^p
            let xp = FieldElement::exp(x, *incremental_degree);
            result = result + result_adj * xp;
        }

        result
    }

    fn build_coefficients<C: PublicCoin>(coin: &C) -> (Vec<FieldElement>, Vec<FieldElement>) {
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
        for evaluations in self.t_evaluation_table.iter() {
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
