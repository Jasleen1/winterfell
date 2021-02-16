use prover::{
    crypto::RandomElementGenerator,
    math::{
        field::{BaseElement, FieldElement, FromVec},
        polynom,
    },
    ComputationContext, ConstraintDegree, TransitionConstraintGroup, TransitionEvaluator,
};

use crate::utils::{
    are_equal, build_cyclic_domain, extend_cyclic_values, is_binary, is_zero, not, transpose,
    EvaluationResult,
};

use super::{rescue, CYCLE_LENGTH as HASH_CYCLE_LEN, SIG_CYCLE_LENGTH as SIG_CYCLE_LEN};

// CONSTANTS
// ================================================================================================
const TWO: BaseElement = BaseElement::new(2);

// RESCUE TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct LamportPlusEvaluator {
    constraint_groups: Vec<TransitionConstraintGroup>,
    sig_mask_poly: Vec<BaseElement>,
    sig_mask_constants: Vec<BaseElement>,
    hash_constants: Vec<Vec<BaseElement>>,
    hash_polys: Vec<Vec<BaseElement>>,
    trace_length: usize,
    trace_generator: BaseElement,
}

impl TransitionEvaluator for LamportPlusEvaluator {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ComputationContext, coeff_prng: RandomElementGenerator) -> Self {
        // build a mask for a signature cycle; the mask has a cycle length of 1024 (that's how
        // many steps are needed to verify a signature), and is defined as 1023 0's followed by 1
        let (sig_mask_poly, sig_mask_constants) = build_sig_cycle_mask(context.ce_blowup_factor());

        let (hash_polys, hash_constants) = build_hash_cycle_constants(context.ce_blowup_factor());

        // define degrees for all transition constraints
        let degrees = vec![
            ConstraintDegree::with_cycles(0, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]), // powers of two
            ConstraintDegree::with_cycles(2, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]), // m0 bit is binary
            ConstraintDegree::with_cycles(2, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]), // m1 bit is binary
            ConstraintDegree::with_cycles(2, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]), // m0 accumulation
            ConstraintDegree::with_cycles(2, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]), // m1 accumulation
            // secret key 1 hashing
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            // secret key 2 hashing
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            // public key hashing
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        ];

        LamportPlusEvaluator {
            constraint_groups: Self::group_constraints(context, &degrees, coeff_prng),
            sig_mask_poly,
            sig_mask_constants,
            hash_polys,
            hash_constants,
            trace_length: context.trace_length(),
            trace_generator: context.generators().trace_domain,
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
        // determine signature mask value for the current and the next steps
        let sig_cycle_start_flag = self.sig_mask_constants[step % self.sig_mask_constants.len()];
        let sig_cycle_end_flag =
            self.sig_mask_constants[(step + 8) % self.sig_mask_constants.len()];

        // determine which rounds constants and masks to use
        let hash_constants = &self.hash_constants[step % self.hash_constants.len()];
        let hash_flag = hash_constants[0];
        let ark = &hash_constants[1..];

        // evaluate constraints with these round constants and masks
        evaluate_constraints(
            result,
            current,
            next,
            ark,
            hash_flag,
            sig_cycle_start_flag,
            sig_cycle_end_flag,
        );
    }

    /// Evaluates transition constraints at the specified x coordinate; this method is
    /// invoked primarily during proof verification.
    fn evaluate_at_x<E: FieldElement + FromVec<BaseElement>>(
        &self,
        result: &mut [E],
        current: &[E],
        next: &[E],
        x: E,
    ) {
        // sig cycle flags
        let num_cycles = (self.trace_length / SIG_CYCLE_LEN) as u32;

        let xp = E::exp(x, num_cycles.into());
        let sig_cycle_start_flag = polynom::eval(&E::from_vec(&self.sig_mask_poly), xp);

        let xp = E::exp(x * E::from(self.trace_generator), num_cycles.into());
        let sig_cycle_end_flag = polynom::eval(&E::from_vec(&self.sig_mask_poly), xp);

        // map x to the corresponding coordinate in constant cycles
        let num_cycles = (self.trace_length / HASH_CYCLE_LEN) as u32;
        let x = E::exp(x, num_cycles.into());

        // determine round constants at the specified x coordinate; we do this by
        // evaluating polynomials for round constants the augmented x coordinate
        let mut hash_constants = [E::ZERO; 1 + 2 * rescue::STATE_WIDTH];
        for (i, poly) in self.hash_polys.iter().enumerate() {
            hash_constants[i] = polynom::eval(&E::from_vec(poly), x);
        }
        let hash_flag = hash_constants[0];
        let ark = &hash_constants[1..];

        // evaluate constraints with these round constants and masks
        evaluate_constraints(result, current, next, ark, hash_flag, sig_cycle_start_flag, sig_cycle_end_flag);
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
fn evaluate_constraints<E: FieldElement + From<BaseElement>>(
    result: &mut [E],
    current: &[E],
    next: &[E],
    ark: &[E],
    hash_flag: E,
    sig_cycle_start_flag: E,
    sig_cycle_end_flag: E,
) {
    // when hash_flag = 1 (which happens on all steps except steps which are multiples of 8),
    // make sure contents of the first 5 registers are copied over, and for other registers,
    // Rescue constraints are applied separately for hashing secret and public keys
    let flag = not(sig_cycle_end_flag) * hash_flag;
    result.agg_constraint(0, flag, are_equal(current[0], next[0]));
    result.agg_constraint(1, flag, are_equal(current[1], next[1]));
    result.agg_constraint(2, flag, are_equal(current[2], next[2]));
    result.agg_constraint(3, flag, are_equal(current[3], next[3]));
    result.agg_constraint(4, flag, are_equal(current[4], next[4]));
    rescue::enforce_round(&mut result[5..11],  &current[5..11],  &next[5..11],  &ark, flag);
    rescue::enforce_round(&mut result[11..17], &current[11..17], &next[11..17], &ark, flag);
    rescue::enforce_round(&mut result[17..23], &current[17..23], &next[17..23], &ark, flag);

    // when input_flag = 1 (which happens on steps which are multiples of 8)
    let flag = not(sig_cycle_end_flag) * not(hash_flag);
    // make sure the contents of the first register are doubled
    result.agg_constraint(0, flag, are_equal(current[0] * E::from(TWO), next[0]));
    // make sure values inserted into registers 1 and 2 are binary
    result.agg_constraint(1, flag, is_binary(current[1]));
    result.agg_constraint(2, flag, is_binary(current[2]));
    // make sure message values were aggregated correctly in registers 3 and 4
    let next_m0 = current[3] + current[0] * current[1];
    result.agg_constraint(3, flag, are_equal(next_m0, next[3]));
    let next_m1 = current[4] + current[0] * current[2];
    result.agg_constraint(4, flag, are_equal(next_m1, next[4]));

    // registers 7..11 and 13..17 were set to zeros
    result.agg_constraint(5, flag, is_zero(next[7]));
    result.agg_constraint(6, flag, is_zero(next[8]));
    result.agg_constraint(7, flag, is_zero(next[9]));
    result.agg_constraint(8, flag, is_zero(next[10]));
    result.agg_constraint(9, flag, is_zero(next[13]));
    result.agg_constraint(10, flag, is_zero(next[14]));
    result.agg_constraint(11, flag, is_zero(next[15]));
    result.agg_constraint(12, flag, is_zero(next[16]));

    // contents of registers 21 and 22 were copied over to the next step
    result.agg_constraint(13, flag, are_equal(current[21], next[21]));
    result.agg_constraint(14, flag, are_equal(current[22], next[22]));

    // when current bit of m0 = 1, hash of private key 1 (which should be equal to public key)
    // should be injected into the hasher state for public key aggregator
    let m0_bit = current[1];
    result.agg_constraint(15, flag * m0_bit,are_equal(current[17] + current[5], next[17]));
    result.agg_constraint(16, flag * m0_bit, are_equal(current[18] + current[6], next[18]));

    // when current bit of m1 = 1, hash of private key 2 (which should be equal to public key)
    // should be injected into the hasher state for public key aggregator
    let m1_bit = current[2];
    result.agg_constraint(17, flag * m1_bit, are_equal(current[19] + current[11], next[19]));
    result.agg_constraint(18, flag * m1_bit, are_equal(current[20] + current[12], next[20]));
}

fn build_sig_cycle_mask(blowup_factor: usize) -> (Vec<BaseElement>, Vec<BaseElement>) {
    let (inv_twiddles, ev_twiddles) = build_cyclic_domain(SIG_CYCLE_LEN, blowup_factor);
    let mut mask = vec![BaseElement::ZERO; SIG_CYCLE_LEN];
    mask[0] = BaseElement::ONE;
    extend_cyclic_values(&mask, &inv_twiddles, &ev_twiddles)
}

fn build_hash_cycle_constants(
    blowup_factor: usize,
) -> (Vec<Vec<BaseElement>>, Vec<Vec<BaseElement>>) {
    let (inv_twiddles, ev_twiddles) = build_cyclic_domain(HASH_CYCLE_LEN, blowup_factor);

    // create a single vector containing values of hash cycle mask and Rescue round constants
    let mut base_constants = vec![HASH_CYCLE_MASK.to_vec()];
    base_constants.append(&mut rescue::get_round_constants());

    // extend the constants to match constraint evaluation domain
    let mut result_polys = Vec::new();
    let mut result_evaluations = Vec::new();

    for constant in base_constants.into_iter() {
        let (poly, evaluations) = extend_cyclic_values(&constant, &inv_twiddles, &ev_twiddles);
        result_polys.push(poly);
        result_evaluations.push(evaluations);
    }
    let constants = transpose(result_evaluations);
    (result_polys, constants)
}

// MASKS
// ================================================================================================
const HASH_CYCLE_MASK: [BaseElement; HASH_CYCLE_LEN] = [
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ZERO,
];
