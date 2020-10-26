use prover::{
    math::{
        fft,
        field::{FieldElement, StarkField},
        polynom,
    },
    ComputationContext, ConstraintDegree, RandomGenerator, TransitionConstraintGroup,
    TransitionEvaluator,
};

use super::{
    rescue::{apply_inv_mds, apply_mds, apply_sbox, ARK},
    CYCLE_LENGTH, STATE_WIDTH,
};

use crate::utils::{are_equal, extend_cyclic_values, is_zero, transpose, EvaluationResult};

// CONSTANTS
// ================================================================================================

/// Specifies steps on which Rescue transition function is applied.
const CYCLE_MASK: [FieldElement; CYCLE_LENGTH] = [
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ONE,
    FieldElement::ZERO,
    FieldElement::ZERO,
];

// RESCUE TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct RescueEvaluator {
    constraint_groups: Vec<TransitionConstraintGroup>,
    mask_constants: Vec<FieldElement>,
    mask_poly: Vec<FieldElement>,
    ark_constants: Vec<Vec<FieldElement>>,
    ark_polys: Vec<Vec<FieldElement>>,
    trace_length: usize,
}

impl TransitionEvaluator for RescueEvaluator {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ComputationContext, coeff_prng: RandomGenerator) -> Self {
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
        let degrees = vec![ConstraintDegree::with_cycles(3, vec![CYCLE_LENGTH]); 4];

        RescueEvaluator {
            constraint_groups: Self::group_constraints(context, &degrees, coeff_prng),
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
    fn evaluate_at_step(
        &self,
        result: &mut [FieldElement],
        current: &[FieldElement],
        next: &[FieldElement],
        step: usize,
    ) {
        // determine which rounds constants to use
        let ark = &self.ark_constants[step % self.ark_constants.len()];

        // when hash_flag = 1, constraints for Rescue round are enforced
        let hash_flag = self.mask_constants[step % self.mask_constants.len()];
        enforce_rescue_round(result, current, next, &ark, hash_flag);

        // when hash_flag = 0, constraints for copying hash values to the next
        // step are enforced.
        let copy_flag = FieldElement::ONE - hash_flag;
        enforce_hash_copy(result, current, next, copy_flag);
    }

    /// Evaluates transition constraints at the specified x coordinate; this method is
    /// invoked primarily during proof verification.
    fn evaluate_at_x(
        &self,
        result: &mut [FieldElement],
        current: &[FieldElement],
        next: &[FieldElement],
        x: FieldElement,
    ) {
        // map x to the corresponding coordinate in constant cycles
        let num_cycles = (self.trace_length / CYCLE_LENGTH) as u128;
        let x = FieldElement::exp(x, num_cycles);

        // determine round constants at the specified x coordinate; we do this by
        // evaluating polynomials for round constants the augmented x coordinate
        let mut ark = [FieldElement::ZERO; 2 * STATE_WIDTH];
        for (i, poly) in self.ark_polys.iter().enumerate() {
            ark[i] = polynom::eval(poly, x);
        }

        // when hash_flag = 1, constraints for Rescue round are enforced
        let hash_flag = polynom::eval(&self.mask_poly, x);
        enforce_rescue_round(result, current, next, &ark, hash_flag);

        // when hash_flag = 0, constraints for copying hash values to the next
        // step are enforced.
        let copy_flag = FieldElement::ONE - hash_flag;
        enforce_hash_copy(result, current, next, copy_flag);
    }

    fn get_ce_blowup_factor() -> usize {
        4
    }

    // BOILERPLATE
    // --------------------------------------------------------------------------------------------

    fn constraint_groups(&self) -> &[TransitionConstraintGroup] {
        &self.constraint_groups
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// when flag = 1, enforces constraints for a single round of Rescue hash functions
fn enforce_rescue_round(
    result: &mut [FieldElement],
    current: &[FieldElement],
    next: &[FieldElement],
    ark: &[FieldElement],
    flag: FieldElement,
) {
    // compute the state that should result from applying the first half of Rescue round
    // to the current state of the computation
    let mut step1 = [FieldElement::ZERO; STATE_WIDTH];
    step1.copy_from_slice(current);
    apply_sbox(&mut step1);
    apply_mds(&mut step1);
    for i in 0..STATE_WIDTH {
        step1[i] = step1[i] + ark[i];
    }

    // compute the state that should result from applying the inverse for the second
    // half for Rescue round to the next step of the computation
    let mut step2 = [FieldElement::ZERO; STATE_WIDTH];
    step2.copy_from_slice(next);
    for i in 0..STATE_WIDTH {
        step2[i] = step2[i] - ark[STATE_WIDTH + i];
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
fn enforce_hash_copy(
    result: &mut [FieldElement],
    current: &[FieldElement],
    next: &[FieldElement],
    flag: FieldElement,
) {
    result.agg_constraint(0, flag, are_equal(current[0], next[0]));
    result.agg_constraint(1, flag, are_equal(current[1], next[1]));
    result.agg_constraint(2, flag, is_zero(next[2]));
    result.agg_constraint(3, flag, is_zero(next[3]));
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_extension_domain(blowup_factor: usize) -> (Vec<FieldElement>, Vec<FieldElement>) {
    let root = FieldElement::get_root_of_unity(CYCLE_LENGTH.trailing_zeros());
    let inv_twiddles = fft::get_inv_twiddles(root, CYCLE_LENGTH);

    let domain_size = CYCLE_LENGTH * blowup_factor;
    let domain_root = FieldElement::get_root_of_unity(domain_size.trailing_zeros());
    let ev_twiddles = fft::get_twiddles(domain_root, domain_size);

    (inv_twiddles, ev_twiddles)
}

fn transpose_ark() -> Vec<Vec<FieldElement>> {
    let mut constants = Vec::new();
    for _ in 0..(STATE_WIDTH * 2) {
        constants.push(vec![FieldElement::ZERO; CYCLE_LENGTH]);
    }

    #[allow(clippy::needless_range_loop)]
    for i in 0..CYCLE_LENGTH {
        for j in 0..(STATE_WIDTH * 2) {
            constants[j][i] = ARK[i][j];
        }
    }

    constants
}
