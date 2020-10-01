use super::{Assertion, AssertionEvaluator, ConstraintDivisor, ProofContext};
use math::field::{self, add, mul, sub};
use std::collections::BTreeMap;

// INPUT/OUTPUT ASSERTION EVALUATOR
// ================================================================================================

pub struct IoAssertionEvaluator {
    input_assertions: Vec<Assertion>,
    input_coefficients: Vec<u128>,
    output_assertions: Vec<Assertion>,
    output_coefficients: Vec<u128>,
    degree_adjustment: u128,
    divisors: Vec<ConstraintDivisor>,
}

impl AssertionEvaluator for IoAssertionEvaluator {
    const MAX_CONSTRAINTS: usize = 128;

    fn new(context: &ProofContext, assertions: &[Assertion], coefficients: &[u128]) -> Self {
        let (input_assertions, output_assertions) =
            group_assertions(&assertions, context.trace_length(), context.trace_width());

        let i_coefficient_num = input_assertions.len() * 2;
        let input_coefficients = coefficients[..i_coefficient_num].to_vec();

        let o_coefficients_num = output_assertions.len() * 2;
        let output_coefficients =
            coefficients[i_coefficient_num..(i_coefficient_num + o_coefficients_num)].to_vec();

        let divisors = vec![
            ConstraintDivisor::from_assertion(context.get_trace_x_at(0)),
            ConstraintDivisor::from_assertion(context.get_trace_x_at(context.trace_length() - 1)),
        ];

        IoAssertionEvaluator {
            input_assertions,
            output_assertions,
            input_coefficients,
            output_coefficients,
            degree_adjustment: get_constraint_adjustment_degree(
                context.trace_length(),
                context.composition_degree(),
            ),
            divisors,
        }
    }

    fn evaluate(&self, result: &mut [u128], state: &[u128], x: u128) {
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
        result[0] = add(i_result, mul(result_adj, xp));

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
        result[1] = add(f_result, mul(result_adj, xp));
    }

    fn divisors(&self) -> &[ConstraintDivisor] {
        &self.divisors
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn group_assertions(
    assertions: &[Assertion],
    trace_length: usize,
    trace_width: usize,
) -> (Vec<Assertion>, Vec<Assertion>) {
    // BTReeMap ensure that assertions are always stored in consistent order
    let mut inputs = BTreeMap::new();
    let mut outputs = BTreeMap::new();

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
