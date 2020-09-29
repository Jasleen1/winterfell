use super::{
    types::{ConstraintEvaluationTable, ConstraintPoly, LdeDomain, TraceTable},
    utils,
};
use common::{
    stark::{
        AssertionEvaluator, ConstraintDivisor, ConstraintEvaluator, ProofContext,
        TransitionEvaluator,
    },
    utils::uninit_vector,
};
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::{fft, field, polynom};

#[cfg(test)]
mod tests;

// PROCEDURES
// ================================================================================================

/// Evaluates constraints defined by the constraint evaluator against the extended execution trace.
pub fn evaluate_constraints<T: TransitionEvaluator, A: AssertionEvaluator>(
    evaluator: &mut ConstraintEvaluator<T, A>,
    extended_trace: &TraceTable,
    lde_domain: &LdeDomain,
) -> ConstraintEvaluationTable {
    // constraints are evaluated over a constraint evaluation domain. this is an optimization
    // because constraint evaluation domain can be many times smaller than the full LDE domain.
    let ce_domain_size = evaluator.ce_domain_size();

    // allocate space for constraint evaluations
    let mut evaluations: Vec<Vec<u128>> = (0..3).map(|_| uninit_vector(ce_domain_size)).collect();

    // allocate buffers to hold current and next rows of the trace table
    let mut current = vec![0; extended_trace.num_registers()];
    let mut next = vec![0; extended_trace.num_registers()];

    // we already have all the data we need in the extended trace table, but since we are
    // doing evaluations over a much smaller domain, we only need to read a small subset
    // of the data. stride specifies how many rows we can skip over.
    let stride = evaluator.lde_domain_size() / ce_domain_size;
    let blowup_factor = evaluator.lde_blowup_factor();
    let lde_domain = lde_domain.values();
    for i in 0..ce_domain_size {
        // translate steps in the constraint evaluation domain to steps in LDE domain;
        // at the last step, next state wraps around and we actually read the first step again
        let lde_step = i * stride;
        let next_lde_step = (lde_step + blowup_factor) % lde_domain.len();

        // read current and next rows from the execution trace table into the buffers
        // TODO: this currently reads each row from trace table twice, and ideally should be fixed
        extended_trace.copy_row(lde_step, &mut current);
        extended_trace.copy_row(next_lde_step, &mut next);

        // pass the current and next rows of the trace table through the constraint evaluator
        // and record the result in respective arrays
        // TODO: this will be changed once the table structure changes to Vec<Vec<u128>>
        let (t_evaluation, i_evaluation, f_evaluation) =
            evaluator.evaluate_at_step(&current, &next, lde_domain[lde_step], i);
        evaluations[0][i] = t_evaluation;
        evaluations[1][i] = i_evaluation;
        evaluations[2][i] = f_evaluation;
    }

    #[cfg(debug_assertions)]
    evaluator.validate_transition_degrees();

    // build and return constraint evaluation table
    ConstraintEvaluationTable::new(evaluations, evaluator.constraint_divisors())
}

/// Interpolates all constraint evaluations into polynomials and combines all these
/// polynomials into a single polynomial
pub fn build_constraint_poly(
    evaluations: ConstraintEvaluationTable,
    context: &ProofContext,
) -> ConstraintPoly {
    let ce_domain_size = evaluations.domain_size();
    let constraint_poly_degree = context.composition_degree();

    let ce_domain_root = field::get_root_of_unity(ce_domain_size);
    let inv_twiddles = fft::get_inv_twiddles(ce_domain_root, ce_domain_size);

    let mut combined_poly = vec![0; ce_domain_size];

    let divisors = evaluations.divisors().to_vec();
    for (mut evaluations, divisor) in evaluations.into_vec().into_iter().zip(divisors.iter()) {
        fft::interpolate_poly(&mut evaluations, &inv_twiddles, true);
        divide_poly(&mut evaluations, divisor);
        debug_assert_eq!(constraint_poly_degree, polynom::degree_of(&evaluations));
        utils::add_in_place(&mut combined_poly, &evaluations);
    }

    ConstraintPoly::new(combined_poly, constraint_poly_degree)
}

/// Evaluates constraint polynomial over LDE domain and returns the result
pub fn extend_constraint_evaluations(
    constraint_poly: &ConstraintPoly,
    lde_domain: &LdeDomain,
) -> Vec<u128> {
    // first, allocate space for the evaluations and copy polynomial coefficients
    // into the lower part of the vector; the remaining values in the vector must
    // be initialized to 0s
    let mut evaluations = vec![field::ZERO; lde_domain.size()];
    evaluations[..constraint_poly.len()].copy_from_slice(&constraint_poly.coefficients());

    // then use FFT to evaluate the polynomial over LDE domain
    fft::evaluate_poly(&mut evaluations, &lde_domain.twiddles(), true);
    evaluations
}

/// Puts constraint evaluations into a Merkle tree; 2 evaluations per leaf
pub fn build_constraint_tree(evaluations: Vec<u128>, hash_fn: HashFunction) -> MerkleTree {
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

// HELPER FUNCTIONS
// ================================================================================================
fn divide_poly(poly: &mut [u128], divisor: &ConstraintDivisor) {
    if divisor.is_simple() {
        assert!(divisor.degree() == 1, "TODO");
        let value = divisor.numerator()[0].1;
        polynom::syn_div_in_place(poly, value);
    } else {
        let numerator_degree = divisor.numerator()[0].0;

        polynom::syn_div_expanded_in_place(poly, numerator_degree, divisor.exclude());
    }
}
