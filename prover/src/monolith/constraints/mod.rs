use super::types::{ConstraintEvaluationTable, TraceTable};
use crate::{AssertionEvaluator, ConstraintEvaluator, TransitionEvaluator};
use common::utils::uninit_vector;
use crypto::{HashFunction, MerkleTree};

#[cfg(test)]
mod tests;

// PROCEDURES
// ================================================================================================

pub fn evaluate_constraints<T: TransitionEvaluator, A: AssertionEvaluator>(
    evaluator: &ConstraintEvaluator<T, A>,
    trace: &TraceTable,
    lde_domain: &Vec<u128>,
) -> ConstraintEvaluationTable {
    let constraint_domain_size = evaluator.trace_length() * evaluator.max_constraint_degree();

    let mut t_evaluations = uninit_vector(constraint_domain_size);
    let mut i_evaluations = uninit_vector(constraint_domain_size);
    let mut f_evaluations = uninit_vector(constraint_domain_size);

    let mut current = vec![0; trace.num_registers()];
    let mut next = vec![0; trace.num_registers()];

    let stride = evaluator.blowup_factor() / evaluator.max_constraint_degree();
    for i in 0..constraint_domain_size {
        trace.copy_row(i * stride, &mut current);
        trace.copy_row(((i + 1) * stride) % lde_domain.len(), &mut next);
        let (t_evaluation, i_evaluation, f_evaluation) =
            evaluator.evaluate(&current, &next, lde_domain[i * stride], i);
        t_evaluations[i] = t_evaluation;
        i_evaluations[i] = i_evaluation;
        f_evaluations[i] = f_evaluation;
    }

    ConstraintEvaluationTable::new(t_evaluations, i_evaluations, f_evaluations)
}

pub fn build_constraint_poly(_evaluations: ConstraintEvaluationTable) -> Vec<u128> {
    // TODO
    vec![]
}

pub fn extend_constraint_evaluations(
    _constraint_poly: Vec<u128>,
    _lde_twiddles: &[u128],
) -> Vec<u128> {
    // TODO
    vec![]
}

pub fn commit_constraints(_constraint_evaluations: &Vec<u128>, _hash: HashFunction) -> MerkleTree {
    panic!("not implemented!");
}
