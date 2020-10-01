use super::{ConstraintDegree, ProofContext};
use std::collections::HashMap;

// TRANSITION EVALUATOR TRAIT
// ================================================================================================

pub trait TransitionEvaluator {
    const MAX_CONSTRAINTS: usize;
    const MAX_CONSTRAINT_DEGREE: usize;

    fn new(context: &ProofContext, coefficients: &[u128]) -> Self;

    /// Evaluates transition constraints at the specified `step` of the execution trace extended
    /// over constraint evaluation domain. The evaluations are saved into the `results` slice. This
    /// method is used by the prover to evaluate/ constraint for all steps of the execution trace.
    fn evaluate_at_step(&self, result: &mut [u128], current: &[u128], next: &[u128], step: usize);

    /// Evaluates transition constraints at the specified `x` coordinate, which could be in or out
    /// of evaluation domain. The evaluations are saved into the `results` slice. This method is
    /// used by both the prover and the verifier to evaluate constraints at an out-of-domain point.
    fn evaluate_at_x(&self, result: &mut [u128], current: &[u128], next: &[u128], x: u128);

    /// Returns degrees of all individual transition constraints.
    fn degrees(&self) -> &[ConstraintDegree];

    fn composition_coefficients(&self) -> &[u128];
}

// PUBLIC FUNCTIONS
// ================================================================================================

pub fn group_transition_constraints(
    composition_degree: usize,
    degrees: &[ConstraintDegree],
    trace_length: usize,
) -> Vec<(u128, Vec<usize>)> {
    let target_degree = get_constraint_target_degree(trace_length, composition_degree);

    let mut groups = HashMap::new();
    for (i, degree) in degrees.iter().enumerate() {
        let evaluation_degree = degree.get_evaluation_degree(trace_length);
        let incremental_degree = (target_degree - evaluation_degree) as u128;
        let group = groups
            .entry(evaluation_degree)
            .or_insert((incremental_degree, Vec::new()));
        group.1.push(i);
    }

    groups.into_iter().map(|e| e.1).collect()
}

// HELPER FUNCTIONS
// ================================================================================================

/// We want to make sure that once roots are divided out of constraint polynomials,
/// the degree of the resulting polynomial will be exactly equal to the composition_degree.
/// For transition constraints, divisor degree = deg(trace). So, target degree for all
/// transitions constraints is simply: deg(composition) + deg(trace)
fn get_constraint_target_degree(trace_length: usize, composition_degree: usize) -> usize {
    let divisor_degree = trace_length - 1;
    composition_degree + divisor_degree
}
