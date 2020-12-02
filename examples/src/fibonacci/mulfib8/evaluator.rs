use super::NUM_REGISTERS;
use crate::utils::are_equal;
use prover::{
    math::field::{BaseElement, FieldElement},
    ComputationContext, ConstraintDegree, RandomGenerator, TransitionConstraintGroup,
    TransitionEvaluator,
};

// FIBONACCI TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct MulFib8Evaluator {
    constraint_groups: Vec<TransitionConstraintGroup>,
}

impl TransitionEvaluator for MulFib8Evaluator {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ComputationContext, coeff_prng: RandomGenerator) -> Self {
        let degrees = vec![ConstraintDegree::new(2); NUM_REGISTERS];
        MulFib8Evaluator {
            constraint_groups: Self::group_constraints(context, &degrees, coeff_prng),
        }
    }

    // TRANSITION CONSTRAINTS
    // --------------------------------------------------------------------------------------------

    fn evaluate_at_step(
        &self,
        result: &mut [BaseElement],
        current: &[BaseElement],
        next: &[BaseElement],
        _step: usize,
    ) {
        self.evaluate_at_x(result, current, next, BaseElement::ZERO)
    }

    fn evaluate_at_x(
        &self,
        result: &mut [BaseElement],
        current: &[BaseElement],
        next: &[BaseElement],
        _x: BaseElement,
    ) {
        // expected state width is 8 field elements
        debug_assert_eq!(NUM_REGISTERS, current.len());
        debug_assert_eq!(NUM_REGISTERS, next.len());

        // constraints of multiplicative Fibonacci (with 8 registers) which state that:
        // s_{0, i+1} = s_{6, i} * s_{7, i}
        // s_{1, i+1} = s_{7, i} * s_{0, i+1}
        // s_{2, i+1} = s_{0, i+1} * s_{1, i+1}
        // s_{3, i+1} = s_{1, i+1} * s_{2, i+1}
        // s_{4, i+1} = s_{2, i+1} * s_{3, i+1}
        // s_{5, i+1} = s_{3, i+1} * s_{4, i+1}
        // s_{6, i+1} = s_{4, i+1} * s_{5, i+1}
        // s_{7, i+1} = s_{5, i+1} * s_{6, i+1}
        result[0] = are_equal(next[0], current[6] * current[7]);
        result[1] = are_equal(next[1], current[7] * next[0]);
        result[2] = are_equal(next[2], next[0] * next[1]);
        result[3] = are_equal(next[3], next[1] * next[2]);
        result[4] = are_equal(next[4], next[2] * next[3]);
        result[5] = are_equal(next[5], next[3] * next[4]);
        result[6] = are_equal(next[6], next[4] * next[5]);
        result[7] = are_equal(next[7], next[5] * next[6]);
    }

    fn constraint_groups(&self) -> &[TransitionConstraintGroup] {
        &self.constraint_groups
    }

    fn get_ce_blowup_factor() -> usize {
        2
    }
}
