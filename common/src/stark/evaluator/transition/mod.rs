use super::ProofContext;

// TRANSITION EVALUATOR TRAIT
// ================================================================================================

pub trait TransitionEvaluator {
    const MAX_CONSTRAINTS: usize;
    const MAX_CONSTRAINT_DEGREE: usize;

    fn new(context: &ProofContext, coefficients: &[u128]) -> Self;

    fn evaluate(&self, current: &[u128], next: &[u128], step: usize) -> Vec<u128>;
    fn evaluate_at(&self, current: &[u128], next: &[u128], x: u128) -> Vec<u128>;

    fn degrees(&self) -> &[usize];
    fn composition_coefficients(&self) -> &[u128];
}

// PUBLIC FUNCTIONS
// ================================================================================================

pub fn group_transition_constraints(
    composition_degree: usize,
    degrees: &[usize],
    trace_length: usize,
) -> Vec<(u128, Vec<usize>)> {
    let max_constraint_degree = degrees.iter().max().unwrap();

    let mut groups: Vec<_> = (0..max_constraint_degree + 1).map(|_| Vec::new()).collect();

    for (i, &degree) in degrees.iter().enumerate() {
        groups[degree].push(i);
    }

    let target_degree = get_constraint_target_degree(trace_length, composition_degree);

    let mut result = Vec::new();
    for (degree, constraints) in groups.iter().enumerate() {
        if constraints.is_empty() {
            continue;
        }
        let constraint_degree = (trace_length - 1) * degree;
        let incremental_degree = (target_degree - constraint_degree) as u128;
        result.push((incremental_degree, constraints.clone()));
    }

    result
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
