use common::{
    errors::VerifierError,
    proof::{DeepValues, StarkProof},
    Assertion, AssertionEvaluator, ComputationContext, ConstraintEvaluator,
    DefaultAssertionEvaluator, TransitionEvaluator,
};
use common::{CompositionCoefficients, PublicCoin};

use math::field::{FieldElement, StarkField};
use std::marker::PhantomData;

mod channel;
mod fri;

// VERIFIER
// ================================================================================================

pub struct Verifier<T: TransitionEvaluator, A: AssertionEvaluator = DefaultAssertionEvaluator> {
    _marker1: PhantomData<T>,
    _marker2: PhantomData<A>,
}

#[allow(clippy::new_without_default)]
impl<T: TransitionEvaluator, A: AssertionEvaluator> Verifier<T, A> {
    /// Creates a new verifier for a computation defined by generic parameters T and A.
    pub fn new() -> Verifier<T, A> {
        Verifier {
            _marker1: PhantomData,
            _marker2: PhantomData,
        }
    }

    /// Verifies the STARK `proof` attesting the assertions are valid in the context of
    /// the computation described by the verifier.
    pub fn verify(
        &self,
        proof: StarkProof,
        assertions: Vec<Assertion>,
    ) -> Result<bool, VerifierError> {
        // initializes a channel which is used to simulate interaction between the prover
        // and the verifier; the verifier can read the values written by the prover into the
        // channel, and also draws random values which the prover uses during proof construction
        let channel = channel::VerifierChannel::new(proof);

        // reads the computation context from the channel. The context contains basic parameters
        // such as trace length, domain sizes, constraint degrees etc.
        let context = channel.read_context();

        // 1 ----- Compute constraint evaluations at OOD point z ----------------------------------

        // draw a pseudo-random out-of-domain point for DEEP composition
        let z = channel.draw_deep_point();

        // build constraint evaluator
        let evaluator = ConstraintEvaluator::<T, A>::new(&channel, context, assertions)?;

        // evaluate constraints at z
        let deep_values = channel.read_deep_values();
        let constraint_evaluation_at_z = evaluate_constraints_at(
            evaluator,
            &deep_values.trace_at_z1,
            &deep_values.trace_at_z2,
            z,
        );

        // 2 ----- Read queried trace states and constraint evaluations ---------------------------

        // draw pseudo-random query positions
        let query_positions = channel.draw_query_positions();

        // compute LDE domain coordinates for all query positions
        let g_lde = context.generators().lde_domain;
        let x_coordinates: Vec<FieldElement> = query_positions
            .iter()
            .map(|&p| FieldElement::exp(g_lde, p as u128))
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
            trace_states,
            &x_coordinates,
            &deep_values,
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
            .collect::<Vec<FieldElement>>();

        // 4 ----- Verify low-degree proof -------------------------------------------------------------
        // make sure that evaluations we computed in the previous step are in fact evaluations
        // of a polynomial of degree equal to context.deep_composition_degree()
        fri::verify(&context, &channel, &evaluations, &query_positions)
            .map_err(VerifierError::VerificationFailed)
    }
}

// CONSTRAINT EVALUATION
// ================================================================================================

/// TODO: move into ConstraintEvaluator?
pub fn evaluate_constraints_at<T: TransitionEvaluator, A: AssertionEvaluator>(
    mut evaluator: ConstraintEvaluator<T, A>,
    state1: &[FieldElement],
    state2: &[FieldElement],
    x: FieldElement,
) -> FieldElement {
    let evaluations = evaluator.evaluate_at_x(state1, state2, x).to_vec();
    let divisors = evaluator.constraint_divisors();
    debug_assert!(
        divisors.len() == evaluations.len(),
        "number of divisors ({}) does not match the number of evaluations ({})",
        divisors.len(),
        evaluations.len()
    );

    // iterate over evaluations and divide out values implied by the divisors
    let mut result = FieldElement::ZERO;
    for (&evaluation, divisor) in evaluations.iter().zip(divisors.iter()) {
        let z = divisor.evaluate_at(x);
        result = result + evaluation / z;
    }

    result
}

// TRACE COMPOSITION
// ================================================================================================

/// TODO: add comments
fn compose_registers(
    context: &ComputationContext,
    trace_states: &[Vec<FieldElement>],
    x_coordinates: &[FieldElement],
    deep_values: &DeepValues,
    z: FieldElement,
    cc: &CompositionCoefficients,
) -> Vec<FieldElement> {
    let next_z = z * context.generators().trace_domain;

    let trace_at_z1 = &deep_values.trace_at_z1;
    let trace_at_z2 = &deep_values.trace_at_z2;

    // TODO: this is computed in several paces; consolidate
    let composition_degree = context.deep_composition_degree();
    let incremental_degree = (composition_degree - (context.trace_length() - 2)) as u128;

    let mut result = Vec::with_capacity(trace_states.len());
    for (registers, &x) in trace_states.iter().zip(x_coordinates) {
        let mut composition = FieldElement::ZERO;
        for (i, &value) in registers.iter().enumerate() {
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
        let xp = FieldElement::exp(x, incremental_degree);
        composition = composition * cc.trace_degree.0 + composition * xp * cc.trace_degree.1;

        result.push(composition);
    }

    result
}

// CONSTRAINT COMPOSITION
// ================================================================================================

/// TODO: add comments
fn compose_constraints(
    evaluations: Vec<FieldElement>,
    x_coordinates: &[FieldElement],
    z: FieldElement,
    evaluation_at_z: FieldElement,
    cc: &CompositionCoefficients,
) -> Vec<FieldElement> {
    // divide out deep point from the evaluations
    let mut result = Vec::with_capacity(evaluations.len());
    for (evaluation, &x) in evaluations.into_iter().zip(x_coordinates) {
        // compute C(x) = (P(x) - P(z)) / (x - z)
        let composition = (evaluation - evaluation_at_z) / (x - z);
        // multiply by pseudo-random coefficient for linear combination
        result.push(composition * cc.constraints);
    }

    result
}
