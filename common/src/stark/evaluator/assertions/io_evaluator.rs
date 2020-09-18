use super::{Assertion, AssertionEvaluator, TraceInfo};
use math::field::{self, add, mul, sub};
use std::collections::HashMap;

// INPUT/OUTPUT ASSERTION EVALUATOR
// ================================================================================================

pub struct IoAssertionEvaluator {
    input_assertions: Vec<Assertion>,
    input_coefficients: Vec<u128>,
    output_assertions: Vec<Assertion>,
    output_coefficients: Vec<u128>,
    degree_adjustment: u128,
}

impl AssertionEvaluator for IoAssertionEvaluator {
    const MAX_CONSTRAINTS: usize = 128;

    fn new(
        assertions: &[Assertion],
        trace: &TraceInfo,
        composition_degree: usize,
        coefficients: &[u128],
    ) -> Self {
        let (input_assertions, output_assertions) =
            group_assertions(&assertions, trace.length(), trace.width());

        let i_coefficient_num = input_assertions.len() * 2;
        let input_coefficients = coefficients[..i_coefficient_num].to_vec();

        let o_coefficients_num = output_assertions.len() * 2;
        let output_coefficients =
            coefficients[i_coefficient_num..(i_coefficient_num + o_coefficients_num)].to_vec();

        IoAssertionEvaluator {
            input_assertions,
            output_assertions,
            input_coefficients,
            output_coefficients,
            degree_adjustment: get_constraint_adjustment_degree(trace.length(), composition_degree),
        }
    }

    fn evaluate(&self, state: &[u128], x: u128) -> (u128, u128) {
        // compute degree adjustment factor
        let xp = field::exp(x, self.degree_adjustment);

        // 1 ----- compute combination of boundary constraints for the first step -----------------
        let mut i_result = field::ZERO;
        let mut result_adj = field::ZERO;

        let cc = &self.input_coefficients;
        for (i, assertion) in self.input_assertions.iter().enumerate() {
            let value = sub(state[assertion.register()], assertion.value());
            i_result = add(i_result, mul(value, cc[i * 2]));
            result_adj = add(result_adj, mul(value, cc[i * 2 + 1]));
        }

        // raise the degree of adjusted terms and sum all the terms together
        i_result = add(i_result, mul(result_adj, xp));

        // 2 ----- compute combination of boundary constraints for the last step ------------------
        let mut f_result = field::ZERO;
        let mut result_adj = field::ZERO;

        let cc = &self.output_coefficients;
        for (i, assertion) in self.output_assertions.iter().enumerate() {
            let value = sub(state[assertion.register()], assertion.value());
            f_result = add(f_result, mul(value, cc[i * 2]));
            result_adj = add(result_adj, mul(value, cc[i * 2 + 1]));
        }

        // raise the degree of adjusted terms and sum all the terms together
        f_result = add(f_result, mul(result_adj, xp));

        (i_result, f_result)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn group_assertions(
    assertions: &[Assertion],
    trace_length: usize,
    trace_width: usize,
) -> (Vec<Assertion>, Vec<Assertion>) {
    let mut inputs = HashMap::new();
    let mut outputs = HashMap::new();

    // TODO: ideally we should build arrays of tuples (register, value, coefficient); this way
    // we don't need to maintain separate arrays for coefficients
    for assertion in assertions {
        assert!(assertion.register() < trace_width, "invalid register index");
        if assertion.step() == 0 {
            assert!(
                !inputs.contains_key(&assertion.register()),
                "duplicated input assertion"
            );
            inputs.insert(assertion.register(), assertion);
        } else if assertion.step() == trace_length - 1 {
            assert!(
                !outputs.contains_key(&assertion.register()),
                "duplicated output assertion"
            );
            outputs.insert(assertion.register(), assertion);
        } else {
            panic!("assertions against arbitrary steps are not yet supported");
        }
    }

    (
        inputs.into_iter().map(|entry| *entry.1).collect(),
        outputs.into_iter().map(|entry| *entry.1).collect(),
    )
}

/// We want to make sure that once roots are divided out of constraint polynomials,
/// the degree of the resulting polynomial will be exactly equal to the composition_degree.
/// For boundary constraints, divisor is a degree 1 polynomial, and in our case, boundary
/// constraints always have degree = deg(trace). So, the adjustment degree is simply:
/// deg(composition) + deg(divisor) - deg(trace)
fn get_constraint_adjustment_degree(trace_length: usize, composition_degree: usize) -> u128 {
    let divisor_degree = 1;
    let target_degree = composition_degree + divisor_degree;
    let boundary_constraint_degree = trace_length - 1;
    (target_degree - boundary_constraint_degree) as u128
}
