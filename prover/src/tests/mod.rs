use common::stark::{TraceInfo, TransitionEvaluator};
use math::field::{self, add, mul, sub};

pub fn build_fib_trace(length: usize) -> Vec<Vec<u128>> {
    assert!(length.is_power_of_two(), "length must be a power of 2");

    let mut reg1 = vec![field::ONE];
    let mut reg2 = vec![field::ONE];

    for i in 0..(length / 2 - 1) {
        reg1.push(add(reg1[i], reg2[i]));
        reg2.push(add(reg1[i], mul(2, reg2[i])));
    }

    vec![reg1, reg2]
}

pub struct FibEvaluator {
    constraint_degrees: Vec<usize>,
    composition_coefficients: Vec<u128>,
}

impl TransitionEvaluator for FibEvaluator {
    const MAX_CONSTRAINT_DEGREE: usize = 1;
    const MAX_CONSTRAINTS: usize = 2;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(_trace: &TraceInfo, coefficients: &[u128]) -> Self {
        let constraint_degrees = vec![1, 1];
        let composition_coefficients = coefficients[..4].to_vec();

        FibEvaluator {
            constraint_degrees,
            composition_coefficients,
        }
    }

    // TRANSITION CONSTRAINTS
    // --------------------------------------------------------------------------------------------

    fn evaluate(&self, current: &[u128], next: &[u128], _step: usize) -> Vec<u128> {
        self.evaluate_at(current, next, 0)
    }

    fn evaluate_at(&self, current: &[u128], next: &[u128], _x: u128) -> Vec<u128> {
        // expected state width is 2 field elements
        debug_assert_eq!(2, current.len());
        debug_assert_eq!(2, next.len());

        // constraints of Fibonacci sequence which state that:
        // s_{0, i+1} = s_{0, i} + s_{1, i}
        // s_{1, i+1} = s_{0, i} + 2 * s_{1, i}
        vec![
            are_equal(next[0], add(current[0], current[1])),
            are_equal(next[1], add(current[0], mul(2, current[1]))),
        ]
    }

    // BOILERPLATE
    // --------------------------------------------------------------------------------------------
    fn degrees(&self) -> &[usize] {
        &self.constraint_degrees
    }

    fn composition_coefficients(&self) -> &[u128] {
        &self.composition_coefficients
    }
}

fn are_equal(a: u128, b: u128) -> u128 {
    sub(a, b)
}
