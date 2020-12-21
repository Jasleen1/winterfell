use prover::{
    crypto::RandomElementGenerator,
    math::{
        field::{BaseElement, FieldElement, FromVec},
        polynom,
    },
    ComputationContext, ConstraintDegree, TransitionConstraintGroup, TransitionEvaluator,
};

use crate::utils::{
    are_equal, build_cyclic_domain, extend_cyclic_values, is_zero, transpose, EvaluationResult,
};

use super::{rescue, CYCLE_LENGTH, STATE_WIDTH};

// CONSTANTS
// ================================================================================================

/// Specifies steps on which Rescue transition function is applied.
const CYCLE_MASK: [BaseElement; CYCLE_LENGTH] = [
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
];

// RESCUE TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct RescueEvaluator {
    constraint_groups: Vec<TransitionConstraintGroup>,
    mask_constants: Vec<BaseElement>,
    mask_poly: Vec<BaseElement>,
    ark_constants: Vec<Vec<BaseElement>>,
    ark_polys: Vec<Vec<BaseElement>>,
    trace_length: usize,
}

impl TransitionEvaluator for RescueEvaluator {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ComputationContext, coeff_prng: RandomElementGenerator) -> Self {
        let (inv_twiddles, ev_twiddles) =
            build_cyclic_domain(CYCLE_LENGTH, context.ce_blowup_factor());

        // extend the mask constants to match constraint evaluation domain
        let (mask_poly, mask_constants) =
            extend_cyclic_values(&CYCLE_MASK, &inv_twiddles, &ev_twiddles);

        // extend Rescue round constants to match constraint evaluation domain
        let mut ark_polys = Vec::new();
        let mut ark_evaluations = Vec::new();

        for constant in rescue::get_round_constants().into_iter() {
            let (poly, evaluations) = extend_cyclic_values(&constant, &inv_twiddles, &ev_twiddles);
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
        result: &mut [BaseElement],
        current: &[BaseElement],
        next: &[BaseElement],
        step: usize,
    ) {
        // determine which rounds constants to use
        let ark = &self.ark_constants[step % self.ark_constants.len()];

        // when hash_flag = 1, constraints for Rescue round are enforced
        let hash_flag = self.mask_constants[step % self.mask_constants.len()];
        rescue::enforce_round(result, current, next, &ark, hash_flag);

        // when hash_flag = 0, constraints for copying hash values to the next
        // step are enforced.
        let copy_flag = BaseElement::ONE - hash_flag;
        enforce_hash_copy(result, current, next, copy_flag);
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
        // map x to the corresponding coordinate in constant cycles
        let num_cycles = (self.trace_length / CYCLE_LENGTH) as u32;
        let x = E::exp(x, num_cycles.into());

        // determine round constants at the specified x coordinate; we do this by
        // evaluating polynomials for round constants the augmented x coordinate
        let mut ark = [E::ZERO; 2 * STATE_WIDTH];
        for (i, poly) in self.ark_polys.iter().enumerate() {
            ark[i] = polynom::eval(&E::from_vec(poly), x);
        }

        // when hash_flag = 1, constraints for Rescue round are enforced
        let hash_flag = polynom::eval(&E::from_vec(&self.mask_poly), x);
        rescue::enforce_round(result, current, next, &ark, hash_flag);

        // when hash_flag = 0, constraints for copying hash values to the next
        // step are enforced.
        let copy_flag = E::ONE - hash_flag;
        enforce_hash_copy(result, current, next, copy_flag);
    }

    fn constraint_groups(&self) -> &[TransitionConstraintGroup] {
        &self.constraint_groups
    }

    fn get_ce_blowup_factor() -> usize {
        4
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// when flag = 1, enforces that the next state of the computation is defined like so:
/// - the first two registers are equal to the values from the previous step
/// - the other two registers are equal to 0
fn enforce_hash_copy<E: FieldElement>(result: &mut [E], current: &[E], next: &[E], flag: E) {
    result.agg_constraint(0, flag, are_equal(current[0], next[0]));
    result.agg_constraint(1, flag, are_equal(current[1], next[1]));
    result.agg_constraint(2, flag, is_zero(next[2]));
    result.agg_constraint(3, flag, is_zero(next[3]));
}
