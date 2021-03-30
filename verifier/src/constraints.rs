use common::{
    AssertionConstraintGroup, Assertions, CompositionCoefficients, ComputationContext,
    ConstraintDivisor, EvaluationFrame, PublicCoin, TransitionEvaluator,
};
use math::field::{BaseElement, FieldElement, FromVec};

// CONSTRAINT EVALUATION
// ================================================================================================

/// Evaluates constraints for the specified frame.
pub fn evaluate_constraints<C, T, E>(
    coin: &C,
    context: &ComputationContext,
    assertions: Assertions,
    ood_frame: &EvaluationFrame<E>,
    x: E,
) -> E
where
    C: PublicCoin,
    T: TransitionEvaluator,
    E: FieldElement + FromVec<BaseElement>,
{
    // ----- evaluate transition constraints ------------------------------------------------------

    // build the divisor for transition constraints; divisors for all transition constraints are
    // the same and hav the form: (x^steps - 1) / (x - x_at_last_step)
    let t_divisor = ConstraintDivisor::from_transition(context);

    // instantiate transition evaluator
    let t_evaluator = T::new(context, coin.get_transition_coefficient_prng());
    // evaluate transition constraints and merge them into a single value
    let mut t_evaluations = vec![E::ZERO; t_evaluator.num_constraints()];
    t_evaluator.evaluate_at_x(&mut t_evaluations, &ood_frame, x);
    let t_evaluation = t_evaluator.merge_evaluations(&t_evaluations, x);

    // divide out the evaluation of divisor at x
    let z = t_divisor.evaluate_at(x);
    let mut result = t_evaluation / z;

    // ----- evaluate assertion constraints -------------------------------------------------------

    // convert assertions into assertion constraints grouped by common divisor
    let assertion_groups =
        assertions.into_constraints(context, coin.get_assertion_coefficient_prng());

    // iterate over assertion constraint groups (each group has a distinct divisor), evaluate
    // constraints in each group and add them to the evaluations vector

    // cache power of x here so that we only re-compute it when degree_adjustment changes
    let mut degree_adjustment = assertion_groups[0].degree_adjustment();
    let mut xp = x.exp(degree_adjustment.into());

    for group in assertion_groups.iter() {
        // if adjustment degree hasn't changed, no need to recompute `xp` - so just reuse the
        // previous value; otherwise, compute new `xp`
        if group.degree_adjustment() != degree_adjustment {
            degree_adjustment = group.degree_adjustment();
            xp = x.exp(degree_adjustment.into());
        }
        // evaluate all constraints in the group, and the divide out the value implied
        // by the divisor
        let evaluation = evaluate_assertion_group(group, &ood_frame.current, x, xp);
        let z = group.divisor().evaluate_at(x);
        result = result + evaluation / z;
    }

    result
}

/// Evaluates a group of assertion constraints at the specified point `x`. All evaluations
/// are combined into a single value via random linear combination. The degree of the
/// evaluation is adjusted based on `xp` parameter.
fn evaluate_assertion_group<E: FieldElement + From<BaseElement>>(
    group: &AssertionConstraintGroup,
    state: &[E],
    x: E,
    xp: E,
) -> E {
    // initialize result aggregators; we use two different aggregators so that we can
    // accumulate the results separately for degree adjusted and un-adjusted terms.
    let mut result = E::ZERO;
    let mut result_adj = E::ZERO;

    // iterate over all constraints in the group, evaluate them, and add the evaluation
    // into result aggregators.
    for constraint in group.constraints().iter() {
        // evaluate the constraint at `x`
        let evaluation = constraint.evaluate_at(x, state[constraint.register()]);
        // then multiply the result by combination coefficients, and add them to the aggregators
        result = result + evaluation * E::from(constraint.cc().0);
        result_adj = result_adj + evaluation * E::from(constraint.cc().1);
    }

    // perform degree adjustment and complete the linear combination
    result + result_adj * xp
}

// CONSTRAINT COMPOSITION
// ================================================================================================

/// TODO: add comments
pub fn compose_constraints<E: FieldElement + From<BaseElement>>(
    evaluations: Vec<BaseElement>,
    x_coordinates: &[BaseElement],
    z: E,
    evaluation_at_z: E,
    cc: &CompositionCoefficients<E>,
) -> Vec<E> {
    // divide out deep point from the evaluations
    let mut result = Vec::with_capacity(evaluations.len());
    for (evaluation, &x) in evaluations.into_iter().zip(x_coordinates) {
        // compute C(x) = (P(x) - P(z)) / (x - z)
        let composition = (E::from(evaluation) - evaluation_at_z) / (E::from(x) - z);
        // multiply by pseudo-random coefficient for linear combination
        result.push(composition * cc.constraints);
    }

    result
}
