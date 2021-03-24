use crate::{errors::*, ComputationContext, PublicCoin};
use math::field::{BaseElement, FieldElement, FromVec};

mod transition;
pub use transition::{TransitionConstraintGroup, TransitionEvaluator};

mod assertions;
pub use assertions::{Assertion, AssertionConstraint, AssertionConstraintGroup, Assertions};

mod constraints;
pub use constraints::{ConstraintDegree, ConstraintDivisor};

mod frame;
pub use frame::EvaluationFrame;

#[cfg(test)]
mod tests;

// CONSTRAINT EVALUATOR
// ================================================================================================

pub struct ConstraintEvaluator<T: TransitionEvaluator> {
    assertions: Vec<AssertionConstraintGroup>,
    transition: T,
    context: ComputationContext,
    divisors: Vec<ConstraintDivisor>,

    #[cfg(debug_assertions)]
    t_evaluation_table: Vec<Vec<BaseElement>>,
}

impl<T: TransitionEvaluator> ConstraintEvaluator<T> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    pub fn new<C: PublicCoin>(
        coin: &C,
        context: &ComputationContext,
        assertions: Assertions,
    ) -> Result<Self, EvaluatorError> {
        if assertions.is_empty() {
            return Err(EvaluatorError::NoAssertionsSpecified);
        }

        // instantiate transition evaluator
        let transition = T::new(context, coin.get_transition_coefficient_prng());

        // determine divisors for all constraints; since divisor for all transition constraints
        // are the same: (x^steps - 1) / (x - x_at_last_step), all transition constraints will be
        // merged into a single value, and the divisor for that value will be first in the list
        let divisors = vec![ConstraintDivisor::from_transition(context)];

        // build assertion constraints
        let assertions = assertions::build_assertion_constraints(
            context,
            assertions,
            coin.get_assertion_coefficient_prng(),
        );

        Ok(ConstraintEvaluator {
            // in debug mode, we keep track of all evaluated transition constraints so that
            // we can verify that their stated degrees match their actual degrees
            #[cfg(debug_assertions)]
            t_evaluation_table: (0..transition.num_constraints())
                .map(|_| Vec::new())
                .collect(),
            transition,
            assertions,
            context: context.clone(),
            divisors,
        })
    }

    // EVALUATION METHODS
    // --------------------------------------------------------------------------------------------

    /// Evaluates transition and assertion constraints at the specified step in the evaluation
    /// domain. This method is used exclusively by the prover because some types of constraints
    /// can be evaluated much more efficiently when the step is known.
    pub fn evaluate_at_step(
        &mut self,
        current: &[BaseElement],
        next: &[BaseElement],
        x: BaseElement,
        step: usize,
    ) -> Result<Vec<BaseElement>, ProverError> {
        let mut evaluations = vec![BaseElement::ZERO; self.divisors.len()];
        let mut t_evaluations = vec![BaseElement::ZERO; self.transition.num_constraints()];

        // evaluate transition constraints and save the results into t_evaluations buffer
        self.transition
            .evaluate_at_step(&mut t_evaluations, current, next, step);

        // when in debug mode, save transition constraint evaluations before merging them
        // so that we can check their degrees later
        #[cfg(debug_assertions)]
        for (i, column) in self.t_evaluation_table.iter_mut().enumerate() {
            column.push(t_evaluations[i]);
        }

        // merge transition constraint evaluations into a single value, and save this value
        // into the first slot of the evaluation buffer. we can do this here because all
        // transition constraints have the same divisor.
        evaluations[0] = self.transition.merge_evaluations(&t_evaluations, x);

        Ok(evaluations)
    }

    /// Evaluates transition and assertion constraints at the specified x coordinate. This
    /// method is used to evaluate constraints at an out-of-domain point. At such a point
    /// there is no `step`, and so the above method cannot be used.
    pub fn evaluate_at_x<E: FieldElement + FromVec<BaseElement>>(
        &mut self,
        current: &[E],
        next: &[E],
        x: E,
    ) -> Vec<E> {
        let mut evaluations = vec![E::ZERO; self.divisors.len()];
        let mut t_evaluations = vec![E::ZERO; self.transition.num_constraints()];

        // evaluate transition constraints and merge them into a single value
        self.transition
            .evaluate_at_x(&mut t_evaluations, current, next, x);
        evaluations[0] = self.transition.merge_evaluations(&t_evaluations, x);

        evaluations
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns length of un-extended execution trace.
    pub fn trace_length(&self) -> usize {
        self.context.trace_length()
    }

    /// Returns size of the constraint evaluation domain.
    pub fn ce_domain_size(&self) -> usize {
        self.context.ce_domain_size()
    }

    /// Returns the generator of the constraint evaluation domain.
    pub fn ce_domain_generator(&self) -> BaseElement {
        self.context.generators().ce_domain
    }

    /// Returns blowup factor for constraint evaluation domain.
    pub fn ce_blowup_factor(&self) -> usize {
        self.context.ce_blowup_factor()
    }

    /// Returns size of low-degree extension domain.
    pub fn lde_domain_size(&self) -> usize {
        self.context.lde_domain_size()
    }

    /// Returns blowup factor for low-degree extension domain.
    pub fn lde_blowup_factor(&self) -> usize {
        self.context.options().blowup_factor()
    }

    /// Returns a list of constraint divisors defined for this evaluator.
    pub fn constraint_divisors(&self) -> &[ConstraintDivisor] {
        &self.divisors
    }

    // Returns a list of assertion constraints for this evaluator.
    pub fn assertion_constraints(&self) -> &[AssertionConstraintGroup] {
        &self.assertions
    }

    // DEBUG HELPERS
    // --------------------------------------------------------------------------------------------

    #[cfg(debug_assertions)]
    pub fn validate_transition_degrees(&mut self) {
        use math::{fft, polynom};

        // collect expected degrees for all transition constraints
        let expected_degrees: Vec<_> = self
            .transition
            .get_constraint_degrees()
            .into_iter()
            .map(|d| d.get_evaluation_degree(self.trace_length()))
            .collect();

        // collect actual degrees for all transition constraints by interpolating saved
        // constraint evaluations into polynomials and checking their degree; also
        // determine max transition constraint degree
        let mut actual_degrees = Vec::with_capacity(expected_degrees.len());
        let mut max_degree = 0;
        let inv_twiddles = fft::get_inv_twiddles::<BaseElement>(self.context.ce_domain_size());
        for evaluations in self.t_evaluation_table.iter() {
            let mut poly = evaluations.clone();
            fft::interpolate_poly(&mut poly, &inv_twiddles);
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
