use prover::{
    math::field::{FieldElement, StarkField},
    ComputationContext, ConstraintDegree, RandomGenerator, TransitionConstraintGroup,
    TransitionEvaluator,
};

use crate::utils::are_equal;

// FIBONACCI TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct MulFib4Evaluator {
    constraint_groups: Vec<TransitionConstraintGroup>,
}

impl TransitionEvaluator for MulFib4Evaluator {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ComputationContext, coeff_prng: RandomGenerator) -> Self {
        let degrees = vec![ConstraintDegree::new(2); 4];
        MulFib4Evaluator {
            constraint_groups: Self::group_constraints(context, &degrees, coeff_prng),
        }
    }

    // TRANSITION CONSTRAINTS
    // --------------------------------------------------------------------------------------------

    fn evaluate_at_step(
        &self,
        result: &mut [FieldElement],
        current: &[FieldElement],
        next: &[FieldElement],
        _step: usize,
    ) {
        self.evaluate_at_x(result, current, next, FieldElement::ZERO)
    }

    fn evaluate_at_x(
        &self,
        result: &mut [FieldElement],
        current: &[FieldElement],
        next: &[FieldElement],
        _x: FieldElement,
    ) {
        // expected state width is 4 field elements
        debug_assert_eq!(4, current.len());
        debug_assert_eq!(4, next.len());

        // constraints of multiplicative Fibonacci (with 4 registers) which state that:
        // s_{0, i+1} = s_{2, i} * s_{3, i}
        // s_{1, i+1} = s_{3, i} * s_{0, i+1}
        // s_{2, i+1} = s_{0, i+1} * s_{1, i+1}
        // s_{3, i+1} = s_{1, i+1} * s_{2, i+1}
        result[0] = are_equal(next[0], current[2] * current[3]);
        result[1] = are_equal(next[1], current[3] * next[0]);
        result[2] = are_equal(next[2], next[0] * next[1]);
        result[3] = are_equal(next[3], next[1] * next[2]);
    }

    fn constraint_groups(&self) -> &[TransitionConstraintGroup] {
        &self.constraint_groups
    }

    fn get_ce_blowup_factor() -> usize {
        2
    }
}
