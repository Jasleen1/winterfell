use common::stark::{
    Assertion, AssertionEvaluator, CompositionCoefficients, ConstraintEvaluator, DeepValues,
    ProofContext, PublicCoin, StarkProof, TransitionEvaluator,
};

use math::field::{self, add, div, exp, mul, sub};
use std::marker::PhantomData;

mod fri;

mod public_coin;
use public_coin::VerifierCoin;

mod proof;
use proof::StarkProofImpl;

// VERIFIER
// ================================================================================================

pub struct Verifier<T: TransitionEvaluator, A: AssertionEvaluator> {
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
    pub fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, String> {
        // reads the computation context from the proof. The context contains basic parameters
        // such as trace length, domain sizes, constraint degrees etc.
        let context = proof.read_context();

        // initializes a public coin which is used to simulate interaction between the prover
        // and the verifier
        let coin = proof.init_public_coin(&context);

        // 1 ----- Compute constraint evaluations at OOD point z ----------------------------------

        // draw a pseudo-random out-of-domain point z
        let z = coin.draw_z();

        // build constraint evaluator
        let evaluator = ConstraintEvaluator::<T, A>::new(&coin, &context, assertions);

        // evaluate constraints at z
        let deep_values = proof.read_deep_values();
        let constraint_evaluation_at_z = evaluate_constraints_at(
            evaluator,
            &deep_values.trace_at_z1,
            &deep_values.trace_at_z2,
            z,
        );

        // 2 ----- Read queried trace states and constraint evaluations ---------------------------

        // use the public coin to determine query positions
        let query_positions = coin.draw_query_positions();

        // compute LDE domain coordinates for all query positions
        let g_lde = context.generators().lde_domain;
        let x_coordinates: Vec<u128> = query_positions
            .iter()
            .map(|&p| exp(g_lde, p as u128))
            .collect();

        // read trace states and constraint evaluations at the queried positions; this also
        // checks that Merkle authentication paths for the states and evaluations are valid
        let trace_states = proof.read_trace_states(&query_positions)?;
        let constraint_evaluations = proof.read_constraint_evaluations(&query_positions)?;

        // 3 ----- Compute composition polynomial evaluations -------------------------------------

        // draw coefficients for computing random linear combination of trace and constraint
        // polynomials; the result of this linear combination are evaluations of deep composition
        // polynomial
        let coefficients = coin.draw_composition_coefficients();

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
            .map(|(&t, c)| add(t, c))
            .collect::<Vec<u128>>();

        // 4 ----- Verify low-degree proof -------------------------------------------------------------
        // make sure that evaluations we computed in the previous step are in fact evaluations
        // of a polynomial of degree equal to deep_composition_degree
        match fri::verify(
            &context,
            proof.read_fri_proof(),
            &evaluations,
            &query_positions,
        ) {
            Ok(result) => Ok(result),
            Err(msg) => Err(format!("verification of low-degree proof failed: {}", msg)),
        }
    }
}

// CONSTRAINT EVALUATION
// ================================================================================================

/// TODO: move into ConstraintEvaluator?
pub fn evaluate_constraints_at<T: TransitionEvaluator, A: AssertionEvaluator>(
    evaluator: ConstraintEvaluator<T, A>,
    state1: &[u128],
    state2: &[u128],
    x: u128,
) -> u128 {
    let (t_value, i_value, f_value) = evaluator.evaluate_at(state1, state2, x);

    // Z(x) = x - 1
    let z = sub(x, field::ONE);
    let mut result = div(i_value, z);

    // Z(x) = x - x_at_last_step
    let z = sub(x, evaluator.get_x_at_last_step());
    result = add(result, div(f_value, z));

    // Z(x) = (x^steps - 1) / (x - x_at_last_step)
    let z = div(sub(exp(x, evaluator.trace_length() as u128), field::ONE), z);
    result = add(result, div(t_value, z));

    result
}

// TRACE COMPOSITION
// ================================================================================================

/// TODO: add comments
fn compose_registers(
    context: &ProofContext,
    trace_states: Vec<Vec<u128>>,
    x_coordinates: &[u128],
    deep_values: &DeepValues,
    z: u128,
    cc: &CompositionCoefficients,
) -> Vec<u128> {
    let next_z = mul(z, context.generators().trace_domain);

    let trace_at_z1 = &deep_values.trace_at_z1;
    let trace_at_z2 = &deep_values.trace_at_z2;

    // TODO: this is computed in several paces; consolidate
    let composition_degree = context.deep_composition_degree();
    let incremental_degree = (composition_degree - (context.trace_length() - 2)) as u128;

    let mut result = Vec::with_capacity(trace_states.len());
    for (registers, &x) in trace_states.iter().zip(x_coordinates) {
        let mut composition = field::ZERO;
        for (i, &value) in registers.iter().enumerate() {
            // compute T1(x) = (T(x) - T(z)) / (x - z)
            let t1 = div(sub(value, trace_at_z1[i]), sub(x, z));
            // multiply it by a pseudo-random coefficient, and combine with result
            composition = add(composition, mul(t1, cc.trace1[i]));

            // compute T2(x) = (T(x) - T(z * g)) / (x - z * g)
            let t2 = div(sub(value, trace_at_z2[i]), sub(x, next_z));
            // multiply it by a pseudo-random coefficient, and combine with result
            composition = add(composition, mul(t2, cc.trace2[i]));
        }

        // raise the degree to match composition degree
        let xp = exp(x, incremental_degree);
        let adj_composition = mul(mul(composition, xp), cc.t2_degree);
        composition = add(mul(composition, cc.t1_degree), adj_composition);

        result.push(composition);
    }

    result
}

// CONSTRAINT COMPOSITION
// ================================================================================================

/// TODO: add comments
fn compose_constraints(
    evaluations: Vec<u128>,
    x_coordinates: &[u128],
    z: u128,
    evaluation_at_z: u128,
    cc: &CompositionCoefficients,
) -> Vec<u128> {
    // divide out deep point from the evaluations
    let mut result = Vec::with_capacity(evaluations.len());
    for (evaluation, &x) in evaluations.into_iter().zip(x_coordinates) {
        // compute C(x) = (P(x) - P(z)) / (x - z)
        let composition = div(sub(evaluation, evaluation_at_z), sub(x, z));
        // multiply by pseudo-random coefficient for linear combination
        result.push(mul(composition, cc.constraints));
    }

    result
}
