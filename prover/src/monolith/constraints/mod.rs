use super::types::{ConstraintEvaluationTable, ConstraintPoly, TraceTable};
use crate::{AssertionEvaluator, ConstraintEvaluator, TransitionEvaluator};
use common::utils::uninit_vector;
use crypto::{HashFunction, MerkleTree};
use math::{fft, field, polynom};

use super::utils;

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

    let constraint_domains = evaluator.constraint_domains();
    ConstraintEvaluationTable::new(
        t_evaluations,
        i_evaluations,
        f_evaluations,
        constraint_domains,
    )
}

/// Interpolates all constraint evaluations into polynomials and combines all these
/// polynomials into a single polynomial
pub fn build_constraint_poly(evaluations: ConstraintEvaluationTable) -> ConstraintPoly {
    let trace_length = 8usize; // TODO
    let evaluation_domain_size = 8usize; // TODO
    let x_at_last_step = 1u128; // TODO

    let combination_root = field::get_root_of_unity(evaluation_domain_size);
    let inv_twiddles = fft::get_inv_twiddles(combination_root, evaluation_domain_size);

    // TODO: switch to domain-based evaluation to avoid this type of destructuring
    let mut evaluations = evaluations.into_vec();
    let mut f_evaluations = evaluations.remove(2);
    let mut i_evaluations = evaluations.remove(1);
    let mut t_evaluations = evaluations.remove(0);

    let mut combined_poly = uninit_vector(evaluation_domain_size);

    // 1 ----- boundary constraints for the initial step --------------------------------------
    // interpolate initial step boundary constraint combination into a polynomial, divide the
    // polynomial by Z(x) = (x - 1), and add it to the result
    fft::interpolate_poly(&mut i_evaluations, &inv_twiddles, true);
    polynom::syn_div_in_place(&mut i_evaluations, field::ONE);
    combined_poly.copy_from_slice(&i_evaluations);

    // 2 ----- boundary constraints for the final step ----------------------------------------
    // interpolate final step boundary constraint combination into a polynomial, divide the
    // polynomial by Z(x) = (x - x_at_last_step), and add it to the result
    fft::interpolate_poly(&mut f_evaluations, &inv_twiddles, true);
    polynom::syn_div_in_place(&mut f_evaluations, x_at_last_step);
    utils::add_in_place(&mut combined_poly, &f_evaluations);

    // 3 ----- transition constraints ---------------------------------------------------------
    // interpolate transition constraint combination into a polynomial, divide the polynomial
    // by Z(x) = (x^steps - 1) / (x - x_at_last_step), and add it to the result
    fft::interpolate_poly(&mut t_evaluations, &inv_twiddles, true);
    polynom::syn_div_expanded_in_place(&mut t_evaluations, trace_length, &[x_at_last_step]);
    utils::add_in_place(&mut combined_poly, &t_evaluations);

    ConstraintPoly::new(combined_poly)
}

/// Evaluates constraint polynomial over LDE domain and returns the result
pub fn extend_constraint_evaluations(
    constraint_poly: ConstraintPoly,
    lde_twiddles: &[u128],
) -> Vec<u128> {
    // first, allocate space for the evaluations and copy polynomial coefficients
    // into the lower part of the vector; the remaining values in the vector must
    // be initialized to 0s
    let mut evaluations = vec![field::ZERO; lde_twiddles.len() * 2];
    let constraint_poly = constraint_poly.into_vec();
    evaluations[..constraint_poly.len()].copy_from_slice(&constraint_poly);

    // then use FFT to evaluate the polynomial over LDE domain
    fft::evaluate_poly(&mut evaluations, &lde_twiddles, true);
    evaluations
}

/// Puts constraint evaluations into a Merkle tree; 2 evaluations per leaf
pub fn commit_constraints(evaluations: Vec<u128>, hash_fn: HashFunction) -> MerkleTree {
    // TODO: number of evaluations should be a power of 2 (not just divisible by 2)
    assert!(
        evaluations.len() % 2 == 0,
        "number of values must be divisible by 2"
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
