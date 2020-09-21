use common::stark::{AssertionEvaluator, ConstraintEvaluator, TransitionEvaluator};
use math::field::{self, add, div, exp, sub};

pub fn evaluate_constraints<T: TransitionEvaluator, A: AssertionEvaluator>(
    evaluator: ConstraintEvaluator<T, A>,
    state1: &[u128],
    state2: &[u128],
    x: u128,
) -> u128 {
    let (t_value, i_value, f_value) = evaluator.evaluate_at(state1, state2, x);

    // Z(x) = x - 1
    let z = sub(x, field::ONE);
    let mut result = div(i_value, z);

    // Z(x) = x - x_at_last_step
    let z = sub(x, get_x_at_last_step(evaluator.trace_length()));
    result = add(result, div(f_value, z));

    // Z(x) = (x^steps - 1) / (x - x_at_last_step)
    let z = div(sub(exp(x, evaluator.trace_length() as u128), field::ONE), z);
    result = add(result, div(t_value, z));

    result
}

fn get_x_at_last_step(trace_length: usize) -> u128 {
    let trace_root = field::get_root_of_unity(trace_length);
    exp(trace_root, (trace_length - 1) as u128)
}
