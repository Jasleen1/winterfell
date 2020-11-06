use prover::{
    math::field::{FieldElement, StarkField},
    ComputationContext, ConstraintDegree, RandomGenerator, TransitionConstraintGroup,
    TransitionEvaluator,
};

use crate::utils::are_equal;

// FIBONACCI TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct Fib8Evaluator {
    constraint_groups: Vec<TransitionConstraintGroup>,
}

impl TransitionEvaluator for Fib8Evaluator {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ComputationContext, coeff_prng: RandomGenerator) -> Self {
        let degrees = vec![ConstraintDegree::new(1); 2];
        Fib8Evaluator {
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
        // expected state width is 2 field elements
        debug_assert_eq!(2, current.len());
        debug_assert_eq!(2, next.len());

        // constraints of Fibonacci sequence (2 registers, skipping over 8 terms):
        let n0 = current[0] + current[1];
        let n1 = current[1] + n0;
        let n2 = n0 + n1;
        let n3 = n1 + n2;
        let n4 = n2 + n3;
        let n5 = n3 + n4;
        let n6 = n4 + n5;
        let n7 = n5 + n6;

        result[0] = are_equal(next[0], n6);
        result[1] = are_equal(next[1], n7);
    }

    fn constraint_groups(&self) -> &[TransitionConstraintGroup] {
        &self.constraint_groups
    }

    fn get_ce_blowup_factor() -> usize {
        2
    }
}
