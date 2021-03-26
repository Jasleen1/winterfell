use super::{assertions::AssertionConstraintGroup, ConstraintEvaluationTable};
use crate::monolith::{domain::StarkDomain, trace::TraceTable};
use common::{
    errors::EvaluatorError, Assertions, ComputationContext, ConstraintDivisor, EvaluationFrame,
    PublicCoin, TransitionEvaluator,
};
use math::field::{BaseElement, FieldElement};
use std::collections::HashMap;

// CONSTRAINT EVALUATOR
// ================================================================================================

pub struct ConstraintEvaluator<T>
where
    T: TransitionEvaluator,
{
    assertions: Vec<AssertionConstraintGroup>,
    transition: T,
    context: ComputationContext,
    divisors: Vec<ConstraintDivisor>,

    #[cfg(debug_assertions)]
    transition_constraint_degrees: Vec<usize>,
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

        // collect expected degrees for all transition constraints to compare them against actual
        // degrees; we do this in debug mode only because this comparison is expensive
        #[cfg(debug_assertions)]
        let transition_constraint_degrees = transition
            .get_constraint_degrees()
            .into_iter()
            .map(|d| d.get_evaluation_degree(context.trace_length()))
            .collect();

        // set divisor for transition constraints; since divisors for all transition constraints
        // are the same: (x^steps - 1) / (x - x_at_last_step), all transition constraints will be
        // merged into a single value, and the divisor for that value will be first in the list
        let mut divisors = vec![ConstraintDivisor::from_transition(context)];

        // build assertion constraints and also append divisors for each group of assertion
        // constraints to the divisor list
        let mut twiddle_map = HashMap::new();
        let assertions = assertions
            .into_constraints(context, coin.get_assertion_coefficient_prng())
            .into_iter()
            .map(|group| {
                divisors.push(group.divisor().clone());
                AssertionConstraintGroup::new(group, context, &mut twiddle_map)
            })
            .collect();

        Ok(ConstraintEvaluator {
            transition,
            assertions,
            context: context.clone(),
            divisors,
            #[cfg(debug_assertions)]
            transition_constraint_degrees,
        })
    }

    // EVALUATOR
    // --------------------------------------------------------------------------------------------
    /// Evaluates constraints against the provided extended execution trace. Constraints
    /// are evaluated over a constraint evaluation domain. This is an optimization because
    /// constraint evaluation domain can be many times smaller than the full LDE domain.
    pub fn evaluate(
        &self,
        trace: &TraceTable,
        domain: &StarkDomain,
    ) -> ConstraintEvaluationTable<BaseElement> {
        // allocate space for constraint evaluations; when we are in debug mode, we also allocate
        // memory to hold all transition constraint evaluations (before they are merged into a
        // single value) so that we can check their degree late
        #[cfg(not(debug_assertions))]
        let mut evaluation_table =
            ConstraintEvaluationTable::<BaseElement>::new(&self.context, self.divisors.clone());
        #[cfg(debug_assertions)]
        let mut evaluation_table = ConstraintEvaluationTable::<BaseElement>::new(
            &self.context,
            self.divisors.clone(),
            self.transition_constraint_degrees.to_vec(),
        );

        // initialize buffers to hold trace values and evaluation results at each step
        let mut ev_frame = EvaluationFrame::new(trace.width());
        let mut evaluations = vec![BaseElement::ZERO; evaluation_table.num_columns()];
        let mut t_evaluations = vec![BaseElement::ZERO; self.transition.num_constraints()];

        for step in 0..evaluation_table.domain_size() {
            // translate steps in the constraint evaluation domain to steps in LDE domain
            let (lde_step, x) = domain.ce_step_to_lde_info(step);

            // update evaluation frame buffer with data from the execution trace; this will
            // read current and next rows from the trace into the buffer
            trace.read_frame_into(lde_step, &mut ev_frame);

            // evaluate transition constraints and save the merged result the first slot of the
            // evaluations buffer
            evaluations[0] = self.evaluate_transition(&ev_frame, x, step, &mut t_evaluations);

            // when in debug mode, save transition constraint evaluations
            #[cfg(debug_assertions)]
            evaluation_table.update_transition_evaluations(step, &t_evaluations);

            // evaluate assertion constraints; the results go into remaining slots of the
            // evaluations buffer
            self.evaluate_assertions(&ev_frame.current, x, step, &mut evaluations[1..]);

            // record the result in the evaluation table
            evaluation_table.update_row(step, &evaluations);
        }

        // when in debug mode, make sure expected transition constraint degrees align with
        // actual degrees we got during constraint evaluation
        #[cfg(debug_assertions)]
        evaluation_table.validate_transition_degrees(self.context.trace_length());

        evaluation_table
    }

    // EVALUATION HELPERS
    // --------------------------------------------------------------------------------------------

    /// Evaluates transition constraints at the specified step of the execution trace. `step` is
    /// the step in the constraint evaluation, and `x` is the corresponding domain value. That
    /// is, x = s * g^step, where g is the generator of the constraint evaluation domain, and s
    /// is the domain offset.
    fn evaluate_transition(
        &self,
        frame: &EvaluationFrame<BaseElement>,
        x: BaseElement,
        step: usize,
        evaluations: &mut [BaseElement],
    ) -> BaseElement {
        // TODO: use a more efficient way to zero out memory
        evaluations.fill(BaseElement::ZERO);

        // evaluate transition constraints and save the results into evaluations buffer
        self.transition
            .evaluate_at_step(evaluations, &frame.current, &frame.next, step);

        // merge transition constraint evaluations into a single value and return it;
        // we can do this here because all transition constraints have the same divisor.
        self.transition.merge_evaluations(&evaluations, x)
    }

    /// Evaluates all assertion groups at a specific step of the execution trace. `step` is the
    /// step in the constraint evaluation domain, and `x` is the corresponding domain value.
    /// That is, x = s * g^step, where g is the generator of the constraint evaluation domain,
    /// and s is the domain offset.
    fn evaluate_assertions<E: FieldElement + From<BaseElement>>(
        &self,
        state: &[E],
        x: E,
        step: usize,
        result: &mut [E],
    ) {
        // compute the adjustment degree outside of the group so that we can re-use
        // it for groups which have the same adjustment degree
        let mut degree_adjustment = self.assertions[0].degree_adjustment;
        let mut xp = x.exp(degree_adjustment.into());

        for (group, result) in self.assertions.iter().zip(result.iter_mut()) {
            // recompute adjustment degree only when it has changed
            if group.degree_adjustment != degree_adjustment {
                degree_adjustment = group.degree_adjustment;
                xp = x.exp(degree_adjustment.into());
            }
            // evaluate the group and save the result
            *result = group.evaluate(state, step, x, xp);
        }
    }
}
