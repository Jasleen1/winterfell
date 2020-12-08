use prover::{
    math::{
        field::{BaseElement, FieldElement},
        polynom,
    },
    ComputationContext, ConstraintDegree, RandomGenerator, TransitionConstraintGroup,
    TransitionEvaluator,
};

use crate::utils::{
    are_equal, build_cyclic_domain, extend_cyclic_values, is_binary, is_zero, not, transpose,
    EvaluationResult,
};

use super::{rescue, CYCLE_LENGTH};

// CONSTANTS
// ================================================================================================
const TWO: BaseElement = BaseElement::new(2);

// RESCUE TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct LamportPlusEvaluator {
    constraint_groups: Vec<TransitionConstraintGroup>,
    mask_constants: Vec<Vec<BaseElement>>,
    mask_polys: Vec<Vec<BaseElement>>,
    ark_constants: Vec<Vec<BaseElement>>,
    ark_polys: Vec<Vec<BaseElement>>,
    trace_length: usize,
}

impl TransitionEvaluator for LamportPlusEvaluator {
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

        // define degrees for all transition constraints
        let degrees = vec![
            ConstraintDegree::with_cycles(1, vec![CYCLE_LENGTH]), // powers of two
            ConstraintDegree::with_cycles(2, vec![CYCLE_LENGTH]), // m0 bit is binary
            ConstraintDegree::with_cycles(2, vec![CYCLE_LENGTH]), // m1 bit is binary
            ConstraintDegree::with_cycles(2, vec![CYCLE_LENGTH]), // m0 accumulation
            ConstraintDegree::with_cycles(2, vec![CYCLE_LENGTH]), // m1 accumulation
            // secret key 1 hashing
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            // secret key 2 hashing
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            // public key hashing
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
            ConstraintDegree::with_cycles(5, vec![CYCLE_LENGTH]),
        ];

        LamportPlusEvaluator {
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
    /// invoked primarily during proof verification.
    fn evaluate_at_x(
        &self,
        result: &mut [BaseElement],
        current: &[BaseElement],
        next: &[BaseElement],
        x: BaseElement,
    ) {
        // map x to the corresponding coordinate in constant cycles
        let num_cycles = (self.trace_length / CYCLE_LENGTH) as u128;
        let x = BaseElement::exp(x, num_cycles);

        // determine round constants at the specified x coordinate; we do this by
        // evaluating polynomials for round constants the augmented x coordinate
        let mut ark = [BaseElement::ZERO; 2 * rescue::STATE_WIDTH];
        for (i, poly) in self.ark_polys.iter().enumerate() {
            ark[i] = polynom::eval(poly, x);
        }

        // in the same way, determine masks at the specified coordinate
        let mut masks = [BaseElement::ZERO, BaseElement::ZERO];
        for (i, poly) in self.mask_polys.iter().enumerate() {
            masks[i] = polynom::eval(poly, x);
        }

        // evaluate constraints with these round constants and masks
        evaluate_constraints(result, current, next, &ark, &masks);
    }

    fn constraint_groups(&self) -> &[TransitionConstraintGroup] {
        &self.constraint_groups
    }

    fn get_ce_blowup_factor() -> usize {
        8
    }
}

// HELPER FUNCTIONS
// ================================================================================================

#[rustfmt::skip]
fn evaluate_constraints(
    result: &mut [BaseElement],
    current: &[BaseElement],
    next: &[BaseElement],
    ark: &[BaseElement],
    masks: &[BaseElement],
) {
    // when hash_flag = 1 (which happens on all steps except steps which are multiples of 8),
    // make sure contents of the first 5 registers are copied over, and for other registers,
    // Rescue constraints are applied separately for hashing secret and public keys
    let hash_flag = masks[0];
    result.agg_constraint(0, hash_flag, are_equal(current[0], next[0]));
    result.agg_constraint(1, hash_flag, are_equal(current[1], next[1]));
    result.agg_constraint(2, hash_flag, are_equal(current[2], next[2]));
    result.agg_constraint(3, hash_flag, are_equal(current[3], next[3]));
    result.agg_constraint(4, hash_flag, are_equal(current[4], next[4]));
    rescue::enforce_round(&mut result[5..11],  &current[5..11],  &next[5..11],  &ark, hash_flag);
    rescue::enforce_round(&mut result[11..17], &current[11..17], &next[11..17], &ark, hash_flag);
    rescue::enforce_round(&mut result[17..23], &current[17..23], &next[17..23], &ark, hash_flag);

    // when input_flag = 1 (which happens on steps which are multiples of 8)
    let input_flag = not(hash_flag);
    // make sure the contents of the first register are doubled
    result.agg_constraint(0, input_flag, are_equal(current[0] * TWO, next[0]));
    // make sure values inserted into registers 1 and 2 are binary
    result.agg_constraint(1, input_flag, is_binary(current[1]));
    result.agg_constraint(2, input_flag, is_binary(current[2]));
    // make sure message values were aggregated correctly in registers 3 and 4
    let next_m0 = current[3] + current[0] * current[1];
    result.agg_constraint(3, input_flag, are_equal(next_m0, next[3]));
    let next_m1 = current[4] + current[0] * current[2];
    result.agg_constraint(4, input_flag, are_equal(next_m1, next[4]));

    // registers 7..11 and 13..17 were set to zeros
    result.agg_constraint(5, input_flag, is_zero(next[7]));
    result.agg_constraint(6, input_flag, is_zero(next[8]));
    result.agg_constraint(7, input_flag, is_zero(next[9]));
    result.agg_constraint(8, input_flag, is_zero(next[10]));
    result.agg_constraint(9, input_flag, is_zero(next[13]));
    result.agg_constraint(10, input_flag, is_zero(next[14]));
    result.agg_constraint(11, input_flag, is_zero(next[15]));
    result.agg_constraint(12, input_flag, is_zero(next[16]));

    // contents of registers 21 and 22 were copied over to the next step
    result.agg_constraint(13, input_flag, are_equal(current[21], next[21]));
    result.agg_constraint(14, input_flag, are_equal(current[22], next[22]));

    // when current bit of m0 = 1, hash of private key 1 (which should be equal to public key)
    // should be injected into the hasher state for public key aggregator
    let m0_bit = current[1];
    result.agg_constraint(15, input_flag * m0_bit,are_equal(current[17] + current[5], next[17]));
    result.agg_constraint(16, input_flag * m0_bit, are_equal(current[18] + current[6], next[18]));

    // when current bit of m1 = 1, hash of private key 2 (which should be equal to public key)
    // should be injected into the hasher state for public key aggregator
    let m1_bit = current[2];
    result.agg_constraint(17, input_flag * m1_bit, are_equal(current[19] + current[11], next[19]));
    result.agg_constraint(18, input_flag * m1_bit, are_equal(current[20] + current[12], next[20]));
}

// MASKS
// ================================================================================================

const CYCLE_MASKS: [[BaseElement; CYCLE_LENGTH]; 1] = [[
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ZERO,
]];
