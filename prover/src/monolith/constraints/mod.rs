use super::{
    types::{ConstraintEvaluationTable, ConstraintPoly, LdeDomain, TraceTable},
    utils,
};
use common::{
    errors::ProverError, utils::uninit_vector, ComputationContext, ConstraintDivisor,
    ConstraintEvaluator, TransitionEvaluator,
};
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::{
    fft,
    field::{BaseElement, FieldElement, FromVec},
    polynom,
};

mod assertions;
use assertions::{evaluate_assertions, prepare_assertion_constraints};

#[cfg(test)]
mod tests;

// PROCEDURES
// ================================================================================================

/// Evaluates constraints defined by the constraint evaluator against the extended execution trace.
pub fn evaluate_constraints<T: TransitionEvaluator, E: FieldElement + FromVec<BaseElement>>(
    evaluator: &mut ConstraintEvaluator<T>,
    extended_trace: &TraceTable,
    lde_domain: &LdeDomain,
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
        let mut evaluation_row =
            evaluator.evaluate_at_step(&current, &next, lde_domain[lde_step], i)?;

        // evaluate assertion constraints
        evaluate_assertions(
            &assertion_constraints,
            &current,
            lde_domain[lde_step],
            i,
            &mut evaluation_row,
        );

        // record the result in the evaluation table
        for (j, &evaluation) in evaluation_row.iter().enumerate() {
            evaluation_table[j][i] = E::from(evaluation);
        }
    }

    #[cfg(debug_assertions)]
    evaluator.validate_transition_degrees();

    // build and return constraint evaluation table
    Ok(ConstraintEvaluationTable::new(evaluation_table, divisors))
}

/// Interpolates all constraint evaluations into polynomials, divides them by their respective
/// divisors, and combines the results into a single polynomial
pub fn build_constraint_poly<E: FieldElement + FromVec<BaseElement>>(
    evaluations: ConstraintEvaluationTable<E>,
    context: &ComputationContext,
) -> Result<ConstraintPoly<E>, ProverError> {
    let ce_domain_size = context.ce_domain_size();
    let constraint_poly_degree = context.composition_degree();
    let inv_twiddles = fft::get_inv_twiddles(context.generators().ce_domain, ce_domain_size);

    // allocate memory for the combined polynomial
    let mut combined_poly = vec![E::ZERO; ce_domain_size];

    // iterate over all columns of the constraint evaluation table
    for (mut evaluations, divisor) in evaluations.into_iter() {
        // interpolate each column into a polynomial
        fft::interpolate_poly(&mut evaluations, &E::from_vec(&inv_twiddles), true);
        // divide the polynomial by its divisor
        divide_poly(&mut evaluations, &divisor);
        // make sure that the post-division degree of the polynomial matches
        // the expected degree, and add it to the combined polynomial
        if cfg!(debug_assertions) && constraint_poly_degree != polynom::degree_of(&evaluations) {
            return Err(ProverError::MismatchedConstraintPolynomialDegree(
                constraint_poly_degree,
                polynom::degree_of(&evaluations),
            ));
        }
        utils::add_in_place(&mut combined_poly, &evaluations);
    }

    Ok(ConstraintPoly::new(combined_poly, constraint_poly_degree))
}

/// Evaluates constraint polynomial over LDE domain and returns the result
pub fn extend_constraint_evaluations<E: FieldElement + FromVec<BaseElement>>(
    constraint_poly: &ConstraintPoly<E>,
    lde_domain: &LdeDomain,
) -> Vec<E> {
    // first, allocate space for the evaluations and copy polynomial coefficients
    // into the lower part of the vector; the remaining values in the vector must
    // be initialized to 0s
    let mut evaluations = vec![E::ZERO; lde_domain.size()];
    evaluations[..constraint_poly.len()].copy_from_slice(&constraint_poly.coefficients());

    // then use FFT to evaluate the polynomial over LDE domain
    fft::evaluate_poly(&mut evaluations, &E::from_vec(lde_domain.twiddles()), true);
    evaluations
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

// HELPER FUNCTIONS
// ================================================================================================
fn divide_poly<E: FieldElement + From<BaseElement>>(poly: &mut [E], divisor: &ConstraintDivisor) {
    let numerator = divisor.numerator();
    assert!(
        numerator.len() == 1,
        "complex divisors are not yet supported"
    );
    assert!(
        divisor.exclude().len() <= 1,
        "multiple exclusion points are not yet supported"
    );

    let numerator = numerator[0];
    let numerator_degree = numerator.0;

    if divisor.exclude().is_empty() {
        polynom::syn_div_in_place(poly, numerator_degree, E::from(numerator.1));
    } else {
        let exception = E::from(divisor.exclude()[0]);
        polynom::syn_div_in_place_with_exception(poly, numerator_degree, exception);
    }
}
