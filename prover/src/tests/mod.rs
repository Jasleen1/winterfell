use common::{
    stark::{
        ConstraintDegree, ProofOptions, RandomGenerator, TransitionConstraintGroup,
        TransitionEvaluator,
    },
    ComputationContext,
};
use crypto::hash::blake3;
use math::field::{FieldElement, StarkField};

pub fn build_fib_trace(length: usize) -> Vec<Vec<FieldElement>> {
    assert!(length.is_power_of_two(), "length must be a power of 2");

    let mut reg1 = vec![FieldElement::ONE];
    let mut reg2 = vec![FieldElement::ONE];

    for i in 0..(length / 2 - 1) {
        reg1.push(reg1[i] + reg2[i]);
        reg2.push(reg1[i] + FieldElement::from(2u8) * reg2[i]);
    }

    vec![reg1, reg2]
}

pub fn build_proof_context(
    trace_length: usize,
    ce_blowup_factor: usize,
    lde_blowup_factor: usize,
) -> ComputationContext {
    let options = ProofOptions::new(32, lde_blowup_factor, 0, blake3);
    ComputationContext::new(2, trace_length, ce_blowup_factor, options)
}

pub struct FibEvaluator {
    constraint_groups: Vec<TransitionConstraintGroup>,
}

impl TransitionEvaluator for FibEvaluator {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ComputationContext, coeff_prng: RandomGenerator) -> Self {
        let degrees = vec![ConstraintDegree::new(1); 2];
        FibEvaluator {
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

        // constraints of Fibonacci sequence which state that:
        // s_{0, i+1} = s_{0, i} + s_{1, i}
        // s_{1, i+1} = s_{0, i} + 2 * s_{1, i}
        result[0] = are_equal(next[0], current[0] + current[1]);
        result[1] = are_equal(next[1], current[0] + FieldElement::from(2u8) * current[1]);
    }

    fn get_ce_blowup_factor() -> usize {
        2
    }

    // BOILERPLATE
    // --------------------------------------------------------------------------------------------
    fn constraint_groups(&self) -> &[TransitionConstraintGroup] {
        &self.constraint_groups
    }
}

fn are_equal(a: FieldElement, b: FieldElement) -> FieldElement {
    a - b
}
