use super::{utils, StarkDomain, TraceTable};
use common::{
    errors::ProverError, utils::uninit_vector, ComputationContext, ConstraintEvaluator,
    TransitionEvaluator,
};
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::field::{BaseElement, FieldElement};

mod assertions;
use assertions::{evaluate_assertions, prepare_assertion_constraints};

mod constraint_poly;
pub use constraint_poly::ConstraintPoly;

mod evaluation_table;
pub use evaluation_table::ConstraintEvaluationTable;

// TODO: re-enable
//#[cfg(test)]
//mod tests;

// PROCEDURES
// ================================================================================================

/// Evaluates constraints defined by the constraint evaluator against the extended execution trace.
pub fn evaluate_constraints<T: TransitionEvaluator, E: FieldElement + From<BaseElement>>(
    evaluator: &mut ConstraintEvaluator<T>,
    extended_trace: &TraceTable,
    domain: &StarkDomain,
    context: &ComputationContext,
) -> Result<ConstraintEvaluationTable<E>, ProverError> {
    // constraints are evaluated over a constraint evaluation domain. this is an optimization
    // because constraint evaluation domain can be many times smaller than the full LDE domain.
    let ce_domain_size = evaluator.ce_domain_size();

    // perform pre-processing of assertion constraints, extract divisors from them, and combine
    // divisors into a single vector of all constraint divisors (assertion constraint divisors)
    // should be appended at the end
    let mut divisors = evaluator.constraint_divisors().to_vec();
    let assertion_constraints = prepare_assertion_constraints(evaluator, &mut divisors);

    // allocate space for constraint evaluations; there should be as many columns in the
    // table as there are divisors
    let mut evaluation_table: Vec<Vec<E>> = divisors
        .iter()
        .map(|_| uninit_vector(ce_domain_size))
        .collect();

    // allocate buffers to hold current and next rows of the trace table
    let mut current = vec![BaseElement::ZERO; extended_trace.num_registers()];
    let mut next = vec![BaseElement::ZERO; extended_trace.num_registers()];

    // we already have all the data we need in the extended trace table, but since we are
    // doing evaluations over a much smaller domain, we only need to read a small subset
    // of the data. stride specifies how many rows we can skip over.
    let stride = evaluator.lde_domain_size() / ce_domain_size;
    let blowup_factor = evaluator.lde_blowup_factor();
    let lde_domain = domain.lde_values();
    for i in 0..ce_domain_size {
        // translate steps in the constraint evaluation domain to steps in LDE domain;
        // at the last step, next state wraps around and we actually read the first step again
        let lde_step = i * stride;
        let next_lde_step = (lde_step + blowup_factor) % lde_domain.len();

        // read current and next rows from the execution trace table into the buffers
        // TODO: this currently reads each row from trace table twice, and ideally should be fixed
        extended_trace.copy_row(lde_step, &mut current);
        extended_trace.copy_row(next_lde_step, &mut next);

        let x = lde_domain[lde_step];

        // pass the current and next rows of the trace table through the constraint evaluator
        let mut evaluation_row = evaluator.evaluate_at_step(&current, &next, x, i)?;

        // evaluate assertion constraints
        evaluate_assertions(&assertion_constraints, &current, x, i, &mut evaluation_row);

        // record the result in the evaluation table
        for (j, &evaluation) in evaluation_row.iter().enumerate() {
            evaluation_table[j][i] = E::from(evaluation);
        }
    }

    #[cfg(debug_assertions)]
    evaluator.validate_transition_degrees();

    // build and return constraint evaluation table
    Ok(ConstraintEvaluationTable::new(
        evaluation_table,
        divisors,
        context,
    ))
}

/// Puts constraint evaluations into a Merkle tree; 2 evaluations per leaf
pub fn build_constraint_tree<E: FieldElement>(
    evaluations: Vec<E>,
    hash_fn: HashFunction,
) -> MerkleTree {
    assert!(
        evaluations.len().is_power_of_two(),
        "number of values must be a power of 2"
    );

    // reinterpret vector of 16-byte values as a vector of 32-byte arrays; this puts
    // pairs of adjacent evaluation values into a single array element
    let mut v = std::mem::ManuallyDrop::new(evaluations);
    let p = v.as_mut_ptr();
    let len = v.len() / 2;
    let cap = v.capacity() / 2;
    let evaluations = unsafe { Vec::from_raw_parts(p as *mut [u8; 32], len, cap) };

    // build Merkle tree out of evaluations
    MerkleTree::new(evaluations, hash_fn)
}

/// Returns constraint evaluations at the specified positions along with Merkle
/// authentication paths from the constraint_tree root to these evaluations.
/// Since evaluations are compressed into a single field element, the are already
/// included in Merkle authentication paths.
pub fn query_constraints(
    constraint_tree: MerkleTree,
    trace_positions: &[usize],
) -> BatchMerkleProof {
    // first, map trace positions to the corresponding positions in the constraint tree;
    // we need to do this because we store 2 constraint evaluations per leaf
    let mut constraint_positions = Vec::with_capacity(trace_positions.len());
    for &position in trace_positions.iter() {
        let cp = position / 2;
        if !constraint_positions.contains(&cp) {
            constraint_positions.push(cp);
        }
    }

    // build Merkle authentication paths to the leaves specified by constraint positions
    constraint_tree.prove_batch(&constraint_positions)
}
