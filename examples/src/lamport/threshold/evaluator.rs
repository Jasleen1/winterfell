use super::{rescue, HASH_CYCLE_LENGTH as HASH_CYCLE_LEN, SIG_CYCLE_LENGTH as SIG_CYCLE_LEN};
use crate::utils::{
    are_equal, build_cyclic_domain, extend_cyclic_values, is_binary, is_zero, not, transpose,
    EvaluationResult,
};
use prover::{
    crypto::RandomElementGenerator,
    math::{
        field::{BaseElement, FieldElement, FromVec},
        polynom,
    },
    ComputationContext, ConstraintDegree, EvaluationFrame, TransitionConstraintGroup,
    TransitionEvaluator,
};

// CONSTANTS
// ================================================================================================
const TWO: BaseElement = BaseElement::new(2);

// RESCUE TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct LamportThresholdEvaluator {
    constraint_groups: Vec<TransitionConstraintGroup>,
    sig_cycle_polys: Vec<Vec<BaseElement>>,
    sig_cycle_constants: Vec<Vec<BaseElement>>,
    hash_polys: Vec<Vec<BaseElement>>,
    hash_constants: Vec<Vec<BaseElement>>,
    trace_length: usize,
}

impl TransitionEvaluator for LamportThresholdEvaluator {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ComputationContext, coeff_prng: RandomElementGenerator) -> Self {
        // build polynomials and evaluation polynomials for constants which span the entire
        // signature cycle (1024 steps); these constants include:
        // * cycle mask which is 1023 zeros followed by 1 one
        // * powers of two which start at 1 and terminate at 2^128
        // TODO: split this into prover vs. verifier flows because the verifier needs only
        // the polynomial, but the prover needs the evaluations.
        let (sig_cycle_polys, sig_cycle_constants) =
            build_sig_cycle_constants(context.ce_blowup_factor(), context.trace_length());

        // build and evaluate polynomials for constants which span hash cycle (8 steps);
        // these constants include:
        // * cycle mask which is 7 ones followed by 1 zero
        // * 12 Rescue round constants
        let (hash_polys, hash_constants) =
            build_hash_cycle_constants(context.ce_blowup_factor(), context.trace_length());

        // define degrees for all transition constraints
        let degrees = vec![
            ConstraintDegree::with_cycles(2, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]), // m0 bit is binary
            ConstraintDegree::with_cycles(2, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]), // m1 bit is binary
            ConstraintDegree::with_cycles(1, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN, SIG_CYCLE_LEN]), // m0 accumulation
            ConstraintDegree::with_cycles(1, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN, SIG_CYCLE_LEN]), // m1 accumulation
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
            // merkle path verification
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            ConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
            // merkle path index
            ConstraintDegree::with_cycles(2, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]), // index bit is binary
            ConstraintDegree::with_cycles(1, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN, SIG_CYCLE_LEN]), // index accumulator
            // signature count
            ConstraintDegree::with_cycles(2, vec![SIG_CYCLE_LEN]), // sig flag is binary
            ConstraintDegree::with_cycles(1, vec![SIG_CYCLE_LEN]), // sig counter
            ConstraintDegree::with_cycles(2, vec![SIG_CYCLE_LEN]),
        ];

        LamportThresholdEvaluator {
            constraint_groups: Self::group_constraints(context, &degrees, coeff_prng),
            sig_cycle_polys,
            sig_cycle_constants,
            hash_polys,
            hash_constants,
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
        frame: &EvaluationFrame<BaseElement>,
        step: usize,
    ) {
        // determine signature mask and power of two value for the current step
        let sig_cycle_end_flag = self.sig_cycle_constants[step % self.sig_cycle_constants.len()][0];
        let power_of_two = self.sig_cycle_constants[step % self.sig_cycle_constants.len()][1];

        // determine which Rescue rounds constants and hash mask to use
        let hash_constants = &self.hash_constants[step % self.hash_constants.len()];
        let hash_flag = hash_constants[0];
        let ark = &hash_constants[1..];

        // evaluate the constraints
        evaluate_constraints(
            result,
            &frame.current,
            &frame.next,
            ark,
            hash_flag,
            sig_cycle_end_flag,
            power_of_two,
        );
    }

    /// Evaluates transition constraints at the specified x coordinate; this method is
    /// invoked primarily during proof verification.
    fn evaluate_at_x<E: FieldElement + FromVec<BaseElement>>(
        &self,
        result: &mut [E],
        frame: &EvaluationFrame<E>,
        x: E,
    ) {
        // determine signature mask and power of two for the specified x
        // first, we need to map x to the corresponding coordinate in the signature cycle
        let num_cycles = (self.trace_length / SIG_CYCLE_LEN) as u32;
        let xp = x.exp(num_cycles.into());

        let sig_cycle_end_flag = polynom::eval(&E::from_vec(&self.sig_cycle_polys[0]), xp);
        let power_of_two = polynom::eval(&E::from_vec(&self.sig_cycle_polys[1]), xp);

        // determine which Rescue rounds constants and hash mask to use
        let num_cycles = (self.trace_length / HASH_CYCLE_LEN) as u32;
        let x = x.exp(num_cycles.into());
        let mut hash_constants = [E::ZERO; 1 + 2 * rescue::STATE_WIDTH];
        for (i, poly) in self.hash_polys.iter().enumerate() {
            hash_constants[i] = polynom::eval(&E::from_vec(poly), x);
        }
        let hash_flag = hash_constants[0];
        let ark = &hash_constants[1..];

        // evaluate the constraints
        evaluate_constraints(
            result,
            &frame.current,
            &frame.next,
            ark,
            hash_flag,
            sig_cycle_end_flag,
            power_of_two,
        );
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
    sig_cycle_end_flag: E,
    power_of_two: E,
) {
    // when hash_flag = 1 (which happens on all steps except steps which are one less than a
    // multiple of 8 - e.g. all steps except for 7, 15, 23 etc.), and we are not on the last step
    // of a signature cycle make sure the contents of registers 0 - 3 and 28, 29 are copied over,
    // and for other registers, Rescue constraints are applied separately for hashing secret and
    // public keys
    let flag = not(sig_cycle_end_flag) * hash_flag;
    result.agg_constraint(0, flag, are_equal(current[0], next[0]));
    result.agg_constraint(1, flag, are_equal(current[1], next[1]));
    result.agg_constraint(2, flag, are_equal(current[2], next[2]));
    result.agg_constraint(3, flag, are_equal(current[3], next[3]));
    rescue::enforce_round(&mut result[4..10],  &current[4..10],  &next[4..10],  &ark, flag);
    rescue::enforce_round(&mut result[10..16], &current[10..16], &next[10..16], &ark, flag);
    rescue::enforce_round(&mut result[16..22], &current[16..22], &next[16..22], &ark, flag);
    rescue::enforce_round(&mut result[22..28], &current[22..28], &next[22..28], &ark, flag);
    result.agg_constraint(28, flag, are_equal(current[28], next[28]));
    result.agg_constraint(29, flag, are_equal(current[29], next[29]));
    

    // when hash_flag = 0 (which happens on steps which are one less than a multiple of 8 - e.g. 7,
    // 15, 23 etc.), and we are not on the last step of a signature cycle:
    let flag = not(sig_cycle_end_flag) * not(hash_flag);
    // make sure values inserted into registers 0 and 1 are binary
    result.agg_constraint(0, flag, is_binary(current[0]));
    result.agg_constraint(1, flag, is_binary(current[1]));
    // make sure message values were aggregated correctly in registers 2 and 3
    let next_m0 = current[2] + current[0] * power_of_two;
    result.agg_constraint(2, flag, are_equal(next_m0, next[2]));
    let next_m1 = current[3] + current[1] * power_of_two;
    result.agg_constraint(3, flag, are_equal(next_m1, next[3]));

    // registers 6..10 and 12..16 were set to zeros
    result.agg_constraint(4, flag, is_zero(next[6]));
    result.agg_constraint(5, flag, is_zero(next[7]));
    result.agg_constraint(6, flag, is_zero(next[8]));
    result.agg_constraint(7, flag, is_zero(next[9]));
    result.agg_constraint(8, flag, is_zero(next[12]));
    result.agg_constraint(9, flag, is_zero(next[13]));
    result.agg_constraint(10, flag, is_zero(next[14]));
    result.agg_constraint(11, flag, is_zero(next[15]));

    // contents of registers 20 and 21 (capacity section of public key hasher state) were
    // copied over to the next step
    result.agg_constraint(12, flag, are_equal(current[20], next[20]));
    result.agg_constraint(13, flag, are_equal(current[21], next[21]));

    // when current bit of m0 = 1, hash of private key 1 (which should be equal to public key)
    // should be injected into the hasher state for public key aggregator
    let m0_bit = current[0];
    result.agg_constraint(14, flag * m0_bit,are_equal(current[16] + current[4], next[16]));
    result.agg_constraint(15, flag * m0_bit, are_equal(current[17] + current[5], next[17]));

    // when current bit of m1 = 1, hash of private key 2 (which should be equal to public key)
    // should be injected into the hasher state for public key aggregator
    let m1_bit = current[1];
    result.agg_constraint(16, flag * m1_bit, are_equal(current[18] + current[10], next[18]));
    result.agg_constraint(17, flag * m1_bit, are_equal(current[19] + current[11], next[19]));

    // when merkle path bit = 1, next values for registers 22 and 23 should come from
    // registers 24 and 25; but when the bit = 0, values should be copied over from 
    // registers 22 and 23; registers 26 and 27 should be reset to zeros.
    let mp_bit = current[28];
    result.agg_constraint(22, flag * not(mp_bit), are_equal(current[22], next[22]));
    result.agg_constraint(23, flag * not(mp_bit), are_equal(current[23], next[23]));
    result.agg_constraint(24, flag * mp_bit, are_equal(current[22], next[24]));
    result.agg_constraint(25, flag * mp_bit, are_equal(current[23], next[25]));
    result.agg_constraint(26, flag, is_zero(next[26]));
    result.agg_constraint(27, flag, is_zero(next[27]));

    // make sure merkle path index bit is binary
    result.agg_constraint(28, flag, is_binary(current[28]));
    // make sure merkle path index aggregator is incremented correctly
    let next_index_agg = current[29] + current[28] * power_of_two;
    result.agg_constraint(29, flag, are_equal(next_index_agg, next[29]));

    // sig flag should be binary and shouldn't change during the signature cycle
    let sig_flag = current[30];
    result.agg_constraint(30, not(sig_cycle_end_flag), are_equal(sig_flag, next[30]));
    result.agg_constraint(30, sig_cycle_end_flag, is_binary(sig_flag));

    // on all steps but the last step of the signature cycle, sig count should be copied
    // over to the next step; on the last step of the signature cycle the next value of 
    // sig count should be set to the previous value, plus the current value of sig flag
    result.agg_constraint(31, not(sig_cycle_end_flag), are_equal(current[31], next[31]));
    result.agg_constraint(31, sig_cycle_end_flag, are_equal(current[31] + sig_flag, next[31]));

    // when sig_count=1, public key computed during signature verification (registers 16 and 17)
    // should be copied to the beginning of Merkle path computation for the aggregated public key
    // (registers 22 and 23); this constraint should be enforced only on the last step of signature
    // verification cycle
    result.agg_constraint(32, sig_cycle_end_flag * sig_flag, are_equal(current[16], next[22]));
    result.agg_constraint(32, sig_cycle_end_flag * sig_flag, are_equal(current[17], next[23]));
}

/// Builds and evaluates polynomials for constants which span the entire signature cycle (1024 steps);
/// Currently, this includes 2 constants:
/// * cycle mask which is 1023 zeros followed by 1 one
/// * powers of two which get incremented every hash cycle
fn build_sig_cycle_constants(
    blowup_factor: usize,
    trace_length: usize,
) -> (Vec<Vec<BaseElement>>, Vec<Vec<BaseElement>>) {
    // build twiddles to use for interpolating and evaluating polynomials
    let (inv_twiddles, ev_twiddles) = build_cyclic_domain(SIG_CYCLE_LEN);

    // build, interpolate, and then evaluate cycle mask
    let mut mask = vec![BaseElement::ZERO; SIG_CYCLE_LEN];
    mask[SIG_CYCLE_LEN - 1] = BaseElement::ONE;
    let (mask_poly, mask_constants) = extend_cyclic_values(
        &mask,
        &inv_twiddles,
        &ev_twiddles,
        blowup_factor,
        trace_length,
    );

    // build powers of two constants
    let mut powers_of_two = vec![BaseElement::ZERO; SIG_CYCLE_LEN];
    let mut current_power_of_two = BaseElement::ONE;
    powers_of_two[0] = BaseElement::ONE;
    for (i, value) in powers_of_two.iter_mut().enumerate().skip(1) {
        // we switch to a new power of two once every 8 steps this is so that a
        // new power of two is available for every hash cycle
        if i % HASH_CYCLE_LEN == 0 {
            current_power_of_two *= TWO;
        }
        *value = current_power_of_two;
    }

    // interpolate and evaluate powers of two
    let (po2_poly, po2_constants) = extend_cyclic_values(
        &powers_of_two,
        &inv_twiddles,
        &ev_twiddles,
        blowup_factor,
        trace_length,
    );
    let polys = vec![mask_poly, po2_poly];

    // transpose evaluations so that values accessed at every step are next to each other
    let constants = transpose(vec![mask_constants, po2_constants]);
    (polys, constants)
}
/// Builds and evaluates polynomials for constant which span a single hash cycle (8 steps);
/// Currently, these constants include:
/// * cycle mask which is 7 ones followed by 1 zero
/// * 12 Rescue round constants
fn build_hash_cycle_constants(
    blowup_factor: usize,
    trace_length: usize,
) -> (Vec<Vec<BaseElement>>, Vec<Vec<BaseElement>>) {
    let (inv_twiddles, ev_twiddles) = build_cyclic_domain(HASH_CYCLE_LEN);

    // create a single vector containing values of hash cycle mask and Rescue round constants
    let mut base_constants = vec![HASH_CYCLE_MASK.to_vec()];
    base_constants.append(&mut rescue::get_round_constants());

    // extend the constants to match constraint evaluation domain
    let mut result_polys = Vec::new();
    let mut result_evaluations = Vec::new();
    for constant in base_constants.into_iter() {
        let (poly, evaluations) = extend_cyclic_values(
            &constant,
            &inv_twiddles,
            &ev_twiddles,
            blowup_factor,
            trace_length,
        );
        result_polys.push(poly);
        result_evaluations.push(evaluations);
    }

    // transpose evaluations so that values accessed at every step are next to each other
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
