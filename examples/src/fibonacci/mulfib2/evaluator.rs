use super::NUM_REGISTERS;
use crate::utils::are_equal;
use prover::{
    math::field::{BaseElement, FieldElement, FromVec},
    ComputationContext, ConstraintDegree, RandomGenerator, TransitionConstraintGroup,
    TransitionEvaluator,
};

// FIBONACCI TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct MulFib2Evaluator {
    constraint_groups: Vec<TransitionConstraintGroup>,
}

impl TransitionEvaluator for MulFib2Evaluator {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ComputationContext, coeff_prng: RandomGenerator) -> Self {
        let degrees = vec![ConstraintDegree::new(2); NUM_REGISTERS];
        MulFib2Evaluator {
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

    fn evaluate_at_x<E: FieldElement<PositiveInteger = u128> + FromVec<BaseElement>>(
        &self,
        result: &mut [E],
        current: &[E],
        next: &[E],
        _x: E,
    ) {
        // expected state width is 2 field elements
        debug_assert_eq!(NUM_REGISTERS, current.len());
        debug_assert_eq!(NUM_REGISTERS, next.len());

        // constraints of multiplicative Fibonacci (with 2 registers) which state that:
        // s_{0, i+1} = s_{0, i} * s_{1, i}
        // s_{1, i+1} = s_{1, i} * s_{0, i+1}
        result[0] = are_equal(next[0], current[0] * current[1]);
        result[1] = are_equal(next[1], current[1] * next[0]);
    }

    fn constraint_groups(&self) -> &[TransitionConstraintGroup] {
        &self.constraint_groups
    }

    fn get_ce_blowup_factor() -> usize {
        2
    }
}
