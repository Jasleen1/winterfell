use common::stark::{ConstraintDegree, ProofContext, TransitionEvaluator};
use math::field::{StarkField, f128::FieldElement};

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

pub struct FibEvaluator {
    constraint_degrees: Vec<ConstraintDegree>,
    composition_coefficients: Vec<FieldElement>,
}

impl TransitionEvaluator for FibEvaluator {
    const MAX_CONSTRAINTS: usize = 2;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(_context: &ProofContext, coefficients: &[FieldElement]) -> Self {
        let degree = ConstraintDegree::new(1);
        FibEvaluator {
            constraint_degrees: vec![degree.clone(), degree],
            composition_coefficients: coefficients[..4].to_vec(),
        }
    }

    // TRANSITION CONSTRAINTS
    // --------------------------------------------------------------------------------------------

    fn evaluate_at_step(&self, result: &mut [FieldElement], current: &[FieldElement], next: &[FieldElement], _step: usize) {
        self.evaluate_at_x(result, current, next, FieldElement::ZERO)
    }

    fn evaluate_at_x(&self, result: &mut [FieldElement], current: &[FieldElement], next: &[FieldElement], _x: FieldElement) {
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
    fn degrees(&self) -> &[ConstraintDegree] {
        &self.constraint_degrees
    }

    fn composition_coefficients(&self) -> &[FieldElement] {
        &self.composition_coefficients
    }
}

fn are_equal(a: FieldElement, b: FieldElement) -> FieldElement {
    a - b
}
