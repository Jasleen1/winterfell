use prover::{
    math::{
        fft,
        field::{self, add, exp, sub},
        polynom,
    },
    ConstraintDegree, ProofContext, TransitionEvaluator,
};

use super::{
    rescue::{apply_inv_mds, apply_mds, apply_sbox, ARK},
    CYCLE_LENGTH, STATE_WIDTH,
};

use crate::utils::{are_equal, extend_cyclic_values, is_zero, transpose, EvaluationResult};

// CONSTANTS
// ================================================================================================

/// Specifies steps on which Rescue transition function is applied.
const CYCLE_MASK: [u128; CYCLE_LENGTH] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0];

// RESCUE TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct RescueEvaluator {
    constraint_degrees: Vec<ConstraintDegree>,
    composition_coefficients: Vec<u128>,
    mask_constants: Vec<u128>,
    mask_poly: Vec<u128>,
    ark_constants: Vec<Vec<u128>>,
    ark_polys: Vec<Vec<u128>>,
    trace_length: usize,
}

impl TransitionEvaluator for RescueEvaluator {
    const MAX_CONSTRAINTS: usize = 4;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ProofContext, coefficients: &[u128]) -> Self {
        let (inv_twiddles, ev_twiddles) = build_extension_domain(context.ce_blowup_factor());

        // extend the mask constants to match constraint evaluation domain
        let (mask_poly, mask_constants) =
            extend_cyclic_values(&CYCLE_MASK, &inv_twiddles, &ev_twiddles);

        // extend Rescue round constants to match constraint evaluation domain
        let mut ark_polys = Vec::new();
        let mut ark_evaluations = Vec::new();

        let constants = transpose_ark();
        for constant in constants.iter() {
            let (poly, evaluations) = extend_cyclic_values(constant, &inv_twiddles, &ev_twiddles);
            ark_polys.push(poly);
            ark_evaluations.push(evaluations);
        }

        // transpose constant values so that all constants for a single round are stored
        // in one vector
        let ark_constants = transpose(ark_evaluations);

        // transition degree is the same for all constraints
        let degree = ConstraintDegree::with_cycles(3, vec![CYCLE_LENGTH]);

        RescueEvaluator {
            constraint_degrees: vec![degree; 4],
            composition_coefficients: coefficients[..8].to_vec(),
            mask_poly,
            mask_constants,
            ark_constants,
            ark_polys,
            trace_length: context.trace_length(),
        }
    }

    // CONSTRAINT EVALUATORS
    // --------------------------------------------------------------------------------------------

    /// Evaluates transition constraints at the specified step; this method is invoked only
    /// during proof generation.
    fn evaluate_at_step(&self, result: &mut [u128], current: &[u128], next: &[u128], step: usize) {
        // determine which rounds constants to use
        let ark = &self.ark_constants[step % self.ark_constants.len()];

        // when hash_flag = 1, constraints for Rescue round are enforced
        let hash_flag = self.mask_constants[step % self.mask_constants.len()];
        enforce_rescue_round(result, current, next, &ark, hash_flag);

        // when hash_flag = 0, constraints for copying hash values to the next
        // step are enforced.
        let copy_flag = sub(field::ONE, hash_flag);
        enforce_hash_copy(result, current, next, copy_flag);
    }

    /// Evaluates transition constraints at the specified x coordinate; this method is
    /// invoked primarily during proof verification.
    fn evaluate_at_x(&self, result: &mut [u128], current: &[u128], next: &[u128], x: u128) {
        // map x to the corresponding coordinate in constant cycles
        let num_cycles = (self.trace_length / CYCLE_LENGTH) as u128;
        let x = exp(x, num_cycles);

        // determine round constants at the specified x coordinate; we do this by
        // evaluating polynomials for round constants the augmented x coordinate
        let mut ark = [field::ZERO; 2 * STATE_WIDTH];
        for (i, poly) in self.ark_polys.iter().enumerate() {
            ark[i] = polynom::eval(poly, x);
        }

        // when hash_flag = 1, constraints for Rescue round are enforced
        let hash_flag = polynom::eval(&self.mask_poly, x);
        enforce_rescue_round(result, current, next, &ark, hash_flag);

        // when hash_flag = 0, constraints for copying hash values to the next
        // step are enforced.
        let copy_flag = sub(field::ONE, hash_flag);
        enforce_hash_copy(result, current, next, copy_flag);
    }

    fn get_ce_blowup_factor() -> usize {
        4
    }

    // BOILERPLATE
    // --------------------------------------------------------------------------------------------
    fn degrees(&self) -> &[ConstraintDegree] {
        &self.constraint_degrees
    }

    fn composition_coefficients(&self) -> &[u128] {
        &self.composition_coefficients
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// when flag = 1, enforces constraints for a single round of Rescue hash functions
fn enforce_rescue_round(
    result: &mut [u128],
    current: &[u128],
    next: &[u128],
    ark: &[u128],
    flag: u128,
) {
    // compute the state that should result from applying the first half of Rescue round
    // to the current state of the computation
    let mut step1 = [0; STATE_WIDTH];
    step1.copy_from_slice(current);
    apply_sbox(&mut step1);
    apply_mds(&mut step1);
    for i in 0..STATE_WIDTH {
        step1[i] = add(step1[i], ark[i]);
    }

    // compute the state that should result from applying the inverse for the second
    // half for Rescue round to the next step of the computation
    let mut step2 = [0; STATE_WIDTH];
    step2.copy_from_slice(next);
    for i in 0..STATE_WIDTH {
        step2[i] = sub(step2[i], ark[STATE_WIDTH + i]);
    }
    apply_inv_mds(&mut step2);
    apply_sbox(&mut step2);

    // make sure that the results are equal
    for i in 0..STATE_WIDTH {
        result.agg_constraint(i, flag, are_equal(step2[i], step1[i]));
    }
}

/// when flag = 1, enforces that the next state of the computation is defined like so:
/// - the first two registers are equal to the values from the previous step
/// - the other two registers are equal to 0
fn enforce_hash_copy(result: &mut [u128], current: &[u128], next: &[u128], flag: u128) {
    result.agg_constraint(0, flag, are_equal(current[0], next[0]));
    result.agg_constraint(1, flag, are_equal(current[1], next[1]));
    result.agg_constraint(2, flag, is_zero(next[2]));
    result.agg_constraint(3, flag, is_zero(next[3]));
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_extension_domain(blowup_factor: usize) -> (Vec<u128>, Vec<u128>) {
    let root = field::get_root_of_unity(CYCLE_LENGTH);
    let inv_twiddles = fft::get_inv_twiddles(root, CYCLE_LENGTH);

    let domain_size = CYCLE_LENGTH * blowup_factor;
    let domain_root = field::get_root_of_unity(domain_size);
    let ev_twiddles = fft::get_twiddles(domain_root, domain_size);

    (inv_twiddles, ev_twiddles)
}

fn transpose_ark() -> Vec<Vec<u128>> {
    let mut constants = Vec::new();
    for _ in 0..(STATE_WIDTH * 2) {
        constants.push(vec![0; CYCLE_LENGTH]);
    }

    #[allow(clippy::needless_range_loop)]
    for i in 0..CYCLE_LENGTH {
        for j in 0..(STATE_WIDTH * 2) {
            constants[j][i] = ARK[i][j];
        }
    }

    constants
}
