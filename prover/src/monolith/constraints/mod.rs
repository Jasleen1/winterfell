use super::{
    types::{ConstraintEvaluationTable, ConstraintPoly, TraceTable},
    utils,
};
use common::{
    stark::{AssertionEvaluator, ConstraintEvaluator, TraceInfo, TransitionEvaluator},
    utils::uninit_vector,
};
use crypto::{HashFunction, MerkleTree};
use math::{fft, field, polynom};

#[cfg(test)]
mod tests;

// PROCEDURES
// ================================================================================================

/// Evaluates constraints defined by the constraint evaluator against the extended execution trace.
pub fn evaluate_constraints<T: TransitionEvaluator, A: AssertionEvaluator>(
    evaluator: &ConstraintEvaluator<T, A>,
    extended_trace: &TraceTable,
    lde_domain: &Vec<u128>,
) -> ConstraintEvaluationTable {
    // constraints are evaluated over a constraint evaluation domain. this is an optimization
    // because constraint evaluation domain can be many times smaller than the full LDE domain.
    let ce_domain_size = evaluator.ce_domain_size();

    // allocate space for constraint evaluations
    // TODO: this should eventually be replaced with Vec<Vec<u128>> so that we don't hard-code
    // order and number of constraint types. but it needs to be done efficiently so that it
    // doesn't affect performance too much
    let mut t_evaluations = uninit_vector(ce_domain_size);
    let mut i_evaluations = uninit_vector(ce_domain_size);
    let mut f_evaluations = uninit_vector(ce_domain_size);

    // allocate buffers to hold current and next rows of the trace table
    let mut current = vec![0; extended_trace.num_registers()];
    let mut next = vec![0; extended_trace.num_registers()];

    // we already have all the data we need in the extended trace table, but since we are
    // doing evaluations over a much smaller domain, we only need to read a small subset
    // of the data. stride specifies how many rows we can skip over.
    let stride = evaluator.lde_domain_size() / ce_domain_size;
    let blowup_factor = evaluator.blowup_factor();
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
            evaluator.evaluate(&current, &next, lde_domain[lde_step], i);
        t_evaluations[i] = t_evaluation;
        i_evaluations[i] = i_evaluation;
        f_evaluations[i] = f_evaluation;
    }

    // build and return constraint evaluation table
    ConstraintEvaluationTable::new(
        t_evaluations,
        i_evaluations,
        f_evaluations,
        evaluator.constraint_divisors(),
    )
}

/// Interpolates all constraint evaluations into polynomials and combines all these
/// polynomials into a single polynomial
pub fn build_constraint_poly(
    evaluations: ConstraintEvaluationTable,
    trace_info: &TraceInfo,
) -> ConstraintPoly {
    let ce_domain_size = evaluations.domain_size();
    let trace_length = trace_info.length();
    let x_at_last_step = get_x_at_last_step(trace_length);

    let ce_domain_root = field::get_root_of_unity(ce_domain_size);
    let inv_twiddles = fft::get_inv_twiddles(ce_domain_root, ce_domain_size);

    // TODO: switch to divisor-based evaluation to avoid this type of destructuring
    let mut evaluations = evaluations.into_vec();
    let mut f_evaluations = evaluations.remove(2);
    let mut i_evaluations = evaluations.remove(1);
    let mut t_evaluations = evaluations.remove(0);

    let mut combined_poly = uninit_vector(ce_domain_size);

    // interpolate initial step boundary constraint combination into a polynomial, divide the
    // polynomial by Z(x) = (x - 1), and add it to the result
    fft::interpolate_poly(&mut i_evaluations, &inv_twiddles, true);
    polynom::syn_div_in_place(&mut i_evaluations, field::ONE);
    combined_poly.copy_from_slice(&i_evaluations);

    // interpolate final step boundary constraint combination into a polynomial, divide the
    // polynomial by Z(x) = (x - x_at_last_step), and add it to the result
    fft::interpolate_poly(&mut f_evaluations, &inv_twiddles, true);
    polynom::syn_div_in_place(&mut f_evaluations, x_at_last_step);
    utils::add_in_place(&mut combined_poly, &f_evaluations);

    // interpolate transition constraint combination into a polynomial, divide the polynomial
    // by Z(x) = (x^steps - 1) / (x - x_at_last_step), and add it to the result
    fft::interpolate_poly(&mut t_evaluations, &inv_twiddles, true);
    polynom::syn_div_expanded_in_place(&mut t_evaluations, trace_length, &[x_at_last_step]);
    utils::add_in_place(&mut combined_poly, &t_evaluations);

    ConstraintPoly::new(combined_poly)
}

/// Evaluates constraint polynomial over LDE domain and returns the result
pub fn extend_constraint_evaluations(
    constraint_poly: &ConstraintPoly,
    lde_twiddles: &[u128],
) -> Vec<u128> {
    // first, allocate space for the evaluations and copy polynomial coefficients
    // into the lower part of the vector; the remaining values in the vector must
    // be initialized to 0s
    let mut evaluations = vec![field::ZERO; lde_twiddles.len() * 2];
    evaluations[..constraint_poly.len()].copy_from_slice(&constraint_poly.coefficients());

    // then use FFT to evaluate the polynomial over LDE domain
    fft::evaluate_poly(&mut evaluations, &lde_twiddles, true);
    evaluations
}

/// Puts constraint evaluations into a Merkle tree; 2 evaluations per leaf
pub fn commit_constraints(evaluations: Vec<u128>, hash_fn: HashFunction) -> MerkleTree {
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

// HELPER FUNCTIONS
// ================================================================================================
fn get_x_at_last_step(trace_length: usize) -> u128 {
    let trace_root = field::get_root_of_unity(trace_length);
    field::exp(trace_root, (trace_length - 1) as u128)
}
