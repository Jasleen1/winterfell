use prover::{
    math::{
        field::{BaseElement, FieldElement, FromVec},
        polynom,
    },
    ComputationContext, ConstraintDegree, RandomGenerator, TransitionConstraintGroup,
    TransitionEvaluator,
};

use super::{rescue, CYCLE_LENGTH, HASH_STATE_WIDTH};

use crate::utils::{
    are_equal, build_cyclic_domain, extend_cyclic_values, is_binary, is_zero, not, transpose,
    EvaluationResult,
};

// RESCUE TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct AnonTokenEvaluator {
    constraint_groups: Vec<TransitionConstraintGroup>,
    mask_constants: Vec<Vec<BaseElement>>,
    mask_polys: Vec<Vec<BaseElement>>,
    ark_constants: Vec<Vec<BaseElement>>,
    ark_polys: Vec<Vec<BaseElement>>,
    trace_length: usize,
}

impl TransitionEvaluator for AnonTokenEvaluator {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ComputationContext, coeff_prng: RandomGenerator) -> Self {
        let (inv_twiddles, ev_twiddles) =
            build_cyclic_domain(CYCLE_LENGTH, context.ce_blowup_factor());

        // extend mask constants to match constraint evaluation domain
        let mut mask_polys = Vec::new();
        let mut mask_evaluations = Vec::new();
        for mask in CYCLE_MASKS.iter() {
            let (poly, evaluations) = extend_cyclic_values(mask, &inv_twiddles, &ev_twiddles);
            mask_polys.push(poly);
            mask_evaluations.push(evaluations);
        }
        let mask_constants = transpose(mask_evaluations);

        // extend Rescue round constants to match constraint evaluation domain
        let mut ark_polys = Vec::new();
        let mut ark_evaluations = Vec::new();

        for constant in rescue::get_round_constants().into_iter() {
            let (poly, evaluations) = extend_cyclic_values(&constant, &inv_twiddles, &ev_twiddles);
            ark_polys.push(poly);
            ark_evaluations.push(evaluations);
        }
        let ark_constants = transpose(ark_evaluations);

        // constraint degree for index bit is just 2 (to check that the bit is 1 or 0)
        // constraint degrees for rescue hash function are 3 + degree of mask;
        // constraint degree for token equality is 1 + degree of mask
        let degrees = vec![
            ConstraintDegree::new(2),
            ConstraintDegree::with_cycles(3, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(3, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(3, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(3, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(3, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(3, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(3, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(3, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(1, vec![CYCLE_LENGTH]),
        ];

        AnonTokenEvaluator {
            constraint_groups: Self::group_constraints(context, &degrees, coeff_prng),
            mask_polys,
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
        result: &mut [BaseElement],
        current: &[BaseElement],
        next: &[BaseElement],
        step: usize,
    ) {
        // determine which rounds constants and masks to use
        let ark = &self.ark_constants[step % self.ark_constants.len()];
        let masks = &self.mask_constants[step % self.mask_constants.len()];

        // evaluate constraints with these round constants and masks
        evaluate_constraints(result, current, next, ark, masks);
    }

    /// Evaluates transition constraints at the specified x coordinate; this method is
    /// invoked only during proof verification.
    fn evaluate_at_x<E: FieldElement<PositiveInteger = u128> + FromVec<BaseElement>>(
        &self,
        result: &mut [E],
        current: &[E],
        next: &[E],
        x: E,
    ) {
        // map x to the corresponding coordinate in constant cycles
        let num_cycles = (self.trace_length / CYCLE_LENGTH) as u128;
        let x = E::exp(x, num_cycles);

        // determine round constants at the specified x coordinate; we do this by
        // evaluating polynomials for round constants the augmented x coordinate
        let mut ark = [E::ZERO; 2 * HASH_STATE_WIDTH];
        for (i, poly) in self.ark_polys.iter().enumerate() {
            ark[i] = polynom::eval(&E::from_vec(poly), x);
        }

        // in the same way, determine masks at the specified coordinate
        let mut masks = [E::ZERO, E::ZERO, E::ZERO];
        for (i, poly) in self.mask_polys.iter().enumerate() {
            masks[i] = polynom::eval(&E::from_vec(poly), x);
        }

        // evaluate constraints with these round constants and masks
        evaluate_constraints(result, current, next, &ark, &masks);
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

fn evaluate_constraints<E: FieldElement<PositiveInteger = u128> + From<BaseElement>>(
    result: &mut [E],
    current: &[E],
    next: &[E],
    ark: &[E],
    masks: &[E],
) {
    // make sure that all values in register 0 are binary (0 or 1)
    let bit = current[0];
    result[0] = is_binary(bit);

    // when hash_flag = 1, constraints for Rescue round are enforced
    let hash_flag = masks[0];
    rescue::enforce_round(
        &mut result[1..5],
        &current[1..5],
        &next[1..5],
        &ark,
        hash_flag,
    );
    rescue::enforce_round(
        &mut result[5..9],
        &current[5..9],
        &next[5..9],
        &ark,
        hash_flag,
    );

    // when hash_flag = 0 and masks[1] = 1, make sure hash results were copied to the next step,
    // and the other two hash state registers were reset to zero
    let result_copy_flag = not(hash_flag) * masks[1];
    result.agg_constraint(1, result_copy_flag, are_equal(current[1], next[1]));
    result.agg_constraint(2, result_copy_flag, are_equal(current[2], next[2]));
    result.agg_constraint(3, result_copy_flag, is_zero(next[3]));
    result.agg_constraint(4, result_copy_flag, is_zero(next[4]));

    // when hash_flag = 0 and masks[1] = 0, make sure accumulated hash was placed in the right
    // place in the hash state for the next round of hashing. Specifically: when index bit = 0
    // accumulated hash must go into registers [1, 2], and when index bit = 0, it must go
    // into registers [3, 4]
    let hash_init_flag = not(hash_flag) * not(masks[1]);
    let not_bit = not(bit);
    result.agg_constraint(1, hash_init_flag, not_bit * are_equal(current[1], next[1]));
    result.agg_constraint(2, hash_init_flag, not_bit * are_equal(current[2], next[2]));
    result.agg_constraint(3, hash_init_flag, bit * are_equal(current[1], next[3]));
    result.agg_constraint(4, hash_init_flag, bit * are_equal(current[2], next[4]));

    // finally, we need to make sure that at steps which are multiples of 16 (e.g. 0, 16, 32 etc.)
    // register[1] == register[5]; technically, we care about this only for step 0, but it is
    // easier to enforce it for all multiples of 16
    let token_cmp_flag = masks[2];
    result.agg_constraint(9, token_cmp_flag, are_equal(current[1], current[5]));
}

// MASKS
// ================================================================================================

const CYCLE_MASKS: [[BaseElement; CYCLE_LENGTH]; 3] = [
    [
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ONE,
        BaseElement::ZERO,
        BaseElement::ZERO,
    ],
    [
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ONE,
        BaseElement::ZERO,
    ],
    [
        BaseElement::ONE,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
    ],
];
