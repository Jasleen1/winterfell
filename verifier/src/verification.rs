use super::{compose_constraints, evaluate_constraints, VerifierChannel};
use common::CompositionCoefficients;
use common::{
    errors::VerifierError, Assertions, ComputationContext, EvaluationFrame, PublicCoin,
    TransitionEvaluator,
};
use fri::VerifierChannel as FriVerifierChannel;
use math::field::{BaseElement, FieldElement, FromVec};

// VERIFICATION PROCEDURE
// ================================================================================================

pub fn perform_verification<T, E>(
    channel: &VerifierChannel<E>,
    assertions: Assertions,
) -> Result<(), VerifierError>
where
    T: TransitionEvaluator,
    E: FieldElement + From<BaseElement> + FromVec<BaseElement>,
{
    let context = channel.context();

    // 1 ----- Compute constraint evaluations at OOD point z ----------------------------------

    // draw a pseudo-random out-of-domain point for DEEP composition
    let z = channel.draw_deep_point::<E>();

    // evaluate constraints at z
    let ood_frame = channel.read_ood_frame()?;
    let constraint_evaluation_at_z = evaluate_constraints::<VerifierChannel<E>, T, E>(
        channel, context, assertions, &ood_frame, z,
    );

    // 2 ----- Read queried trace states and constraint evaluations ---------------------------

    // draw pseudo-random query positions
    let query_positions = channel.draw_query_positions();

    // compute LDE domain coordinates for all query positions
    let g_lde = context.generators().lde_domain;
    let domain_offset = context.domain_offset();
    let x_coordinates: Vec<BaseElement> = query_positions
        .iter()
        .map(|&p| g_lde.exp((p as u64).into()) * domain_offset)
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
        channel.num_fri_partitions(),
        context.options().to_fri_options(),
    );
    fri::verify(&fri_context, channel, &evaluations, &query_positions)
        .map_err(VerifierError::FriVerificationFailed)
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
        let xp = x.exp(incremental_degree.into());
        composition = composition * cc.trace_degree.0 + composition * xp * cc.trace_degree.1;

        result.push(composition);
    }

    result
}
