use super::VerifierChannel;
use common::CompositionCoefficients;
use common::{
    errors::VerifierError, Assertion, AssertionEvaluator, ComputationContext, ConstraintEvaluator,
    EvaluationFrame, PublicCoin, TransitionEvaluator,
};
use math::field::{BaseElement, FieldElement, FromVec};

// VERIFICATION PROCEDURE
// ================================================================================================

pub fn perform_verification<
    T: TransitionEvaluator,
    A: AssertionEvaluator,
    E: FieldElement + From<BaseElement> + FromVec<BaseElement>,
>(
    channel: &VerifierChannel<E>,
    assertions: Vec<Assertion>,
) -> Result<bool, VerifierError> {
    let context = channel.context();

    // 1 ----- Compute constraint evaluations at OOD point z ----------------------------------

    // draw a pseudo-random out-of-domain point for DEEP composition
    let z = channel.draw_deep_point::<E>();

    // build constraint evaluator
    let evaluator = ConstraintEvaluator::<T, A>::new(channel, context, assertions)?;

    // evaluate constraints at z
    let ood_frame = channel.read_ood_frame()?;
    let constraint_evaluation_at_z =
        evaluate_constraints_at(evaluator, &ood_frame.current, &ood_frame.next, z);

    // 2 ----- Read queried trace states and constraint evaluations ---------------------------

    // draw pseudo-random query positions
    let query_positions = channel.draw_query_positions();

    // compute LDE domain coordinates for all query positions
    let g_lde = context.generators().lde_domain;
    let x_coordinates: Vec<BaseElement> = query_positions
        .iter()
        .map(|&p| BaseElement::exp(g_lde, p as u128))
        .collect();

    // read trace states and constraint evaluations at the queried positions; this also
    // checks that Merkle authentication paths for the states and evaluations are valid
    let trace_states = channel.read_trace_states(&query_positions)?;
    let constraint_evaluations = channel.read_constraint_evaluations(&query_positions)?;

    // 3 ----- Compute composition polynomial evaluations -------------------------------------

    // draw coefficients for computing random linear combination of trace and constraint
    // polynomials; the result of this linear combination are evaluations of deep composition
    // polynomial
    let coefficients = channel.draw_composition_coefficients();

    // compute composition of trace registers
    let t_composition = compose_registers(
        &context,
        &trace_states,
        &x_coordinates,
        &ood_frame,
        z,
        &coefficients,
    );

    // compute composition of constraints
    let c_composition = compose_constraints(
        constraint_evaluations,
        &x_coordinates,
        z,
        constraint_evaluation_at_z,
        &coefficients,
    );

    // add the two together
    let evaluations = t_composition
        .iter()
        .zip(c_composition)
        .map(|(&t, c)| t + c)
        .collect::<Vec<_>>();

    // 4 ----- Verify low-degree proof -------------------------------------------------------------
    // make sure that evaluations we computed in the previous step are in fact evaluations
    // of a polynomial of degree equal to context.deep_composition_degree()
    let fri_context = fri::VerifierContext::new(
        context.lde_domain_size(),
        context.composition_degree(),
        context.options().to_fri_options(),
    );
    fri::verify(&fri_context, channel, &evaluations, &query_positions)
        .map_err(VerifierError::VerificationFailed)
}

// CONSTRAINT EVALUATION
// ================================================================================================

/// TODO: move into ConstraintEvaluator?
pub fn evaluate_constraints_at<
    T: TransitionEvaluator,
    A: AssertionEvaluator,
    E: FieldElement + FromVec<BaseElement>,
>(
    mut evaluator: ConstraintEvaluator<T, A>,
    state1: &[E],
    state2: &[E],
    x: E,
) -> E {
    let evaluations = evaluator.evaluate_at_x(state1, state2, x).to_vec();
    let divisors = evaluator.constraint_divisors();
    debug_assert!(
        divisors.len() == evaluations.len(),
        "number of divisors ({}) does not match the number of evaluations ({})",
        divisors.len(),
        evaluations.len()
    );

    // iterate over evaluations and divide out values implied by the divisors
    let mut result = E::ZERO;
    for (&evaluation, divisor) in evaluations.iter().zip(divisors.iter()) {
        let z = divisor.evaluate_at(x);
        result = result + evaluation / z;
    }

    result
}

// TRACE COMPOSITION
// ================================================================================================

/// TODO: add comments
fn compose_registers<E: FieldElement + From<BaseElement>>(
    context: &ComputationContext,
    trace_states: &[Vec<BaseElement>],
    x_coordinates: &[BaseElement],
    ood_frame: &EvaluationFrame<E>,
    z: E,
    cc: &CompositionCoefficients<E>,
) -> Vec<E> {
    let next_z = z * E::from(context.generators().trace_domain);

    let trace_at_z1 = &ood_frame.current;
    let trace_at_z2 = &ood_frame.next;

    // TODO: this is computed in several paces; consolidate
    let composition_degree = context.deep_composition_degree();
    let incremental_degree = (composition_degree - (context.trace_length() - 2)) as u32;

    let mut result = Vec::with_capacity(trace_states.len());
    for (registers, &x) in trace_states.iter().zip(x_coordinates) {
        let x = E::from(x);
        let mut composition = E::ZERO;
        for (i, &value) in registers.iter().enumerate() {
            let value = E::from(value);
            // compute T1(x) = (T(x) - T(z)) / (x - z)
            let t1 = (value - trace_at_z1[i]) / (x - z);
            // multiply it by a pseudo-random coefficient, and combine with result
            composition = composition + t1 * cc.trace[i].0;

            // compute T2(x) = (T(x) - T(z * g)) / (x - z * g)
            let t2 = (value - trace_at_z2[i]) / (x - next_z);
            // multiply it by a pseudo-random coefficient, and combine with result
            composition = composition + t2 * cc.trace[i].1;
        }

        // raise the degree to match composition degree
        let xp = E::exp(x, incremental_degree.into());
        composition = composition * cc.trace_degree.0 + composition * xp * cc.trace_degree.1;

        result.push(composition);
    }

    result
}

// CONSTRAINT COMPOSITION
// ================================================================================================

/// TODO: add comments
fn compose_constraints<E: FieldElement + From<BaseElement>>(
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
