use common::{
    AssertionConstraintGroup, CompositionCoefficients, ConstraintEvaluator, EvaluationFrame,
    TransitionEvaluator,
};
use math::{
    field::{BaseElement, FieldElement, FromVec},
    polynom,
};

// CONSTRAINT EVALUATION
// ================================================================================================

/// Evaluates constraints for the specified frame.
pub fn evaluate_constraints<T: TransitionEvaluator, E: FieldElement + FromVec<BaseElement>>(
    mut evaluator: ConstraintEvaluator<T>,
    ood_frame: &EvaluationFrame<E>,
    x: E,
) -> E {
    let mut result = E::ZERO;

    // evaluate transition constraints
    let evaluations = evaluator.evaluate_at_x(&ood_frame.current, &ood_frame.next, x);
    let divisors = evaluator.constraint_divisors();
    debug_assert!(
        divisors.len() == evaluations.len(),
        "number of divisors ({}) does not match the number of evaluations ({})",
        divisors.len(),
        evaluations.len()
    );

    // iterate over evaluations and divide out values implied by the divisors
    for (&evaluation, divisor) in evaluations.iter().zip(divisors.iter()) {
        let z = divisor.evaluate_at(x);
        result = result + evaluation / z;
    }

    // evaluate assertion constraints
    let assertion_groups = evaluator.assertion_constraints();
    if !assertion_groups.is_empty() {
        let mut degree_adjustment = assertion_groups[0].degree_adjustment();
        let mut xp = E::exp(x, degree_adjustment.into());

        // iterate over assertion constraint groups (each group has a distinct divisor), evaluate
        // constraints in each group and add them to the evaluations vector
        for group in assertion_groups.iter() {
            // if adjustment degree hasn't changed, no need to recompute `xp` - so just reuse the
            // previous value; otherwise, compute new `xp`
            if group.degree_adjustment() != degree_adjustment {
                degree_adjustment = group.degree_adjustment();
                xp = E::exp(x, degree_adjustment.into());
            }
            // evaluate all constraints in the group, and the divide out the value implied
            // by the divisor
            let evaluation = evaluate_assertion_group(group, &ood_frame.current, x, xp);
            let z = group.divisor().evaluate_at(x);
            result = result + evaluation / z;
        }
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
        let value = if constraint.poly.len() == 1 {
            // if constraint polynomial consists of just a constant, use that constant as value
            E::from(constraint.poly[0])
        } else {
            // otherwise, we need to evaluate the polynomial at `x`; but first we need to map
            // the original polynomial into the evaluation field. When we are working in the base
            // field, this has not effect, but when we are working in the extension field,  every
            // coefficient of the polynomial is mapped from the base field into the extension field
            let poly: Vec<E> = constraint.poly.iter().map(|&c| E::from(c)).collect();
            polynom::eval(&poly, x)
        };
        // compute the numerator of the constraint: P(x) - C(x),
        // then multiply the result by combination coefficients, and add them to the aggregators
        let value = state[constraint.register] - value;
        result = result + value * E::from(constraint.cc.0);
        result_adj = result_adj + value * E::from(constraint.cc.1);
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
