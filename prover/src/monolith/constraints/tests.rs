use super::{utils::infer_degree, ConstraintEvaluationTable, TraceInfo};
use crate::{
    monolith::{commit_trace, extend_trace},
    tests::{build_fib_trace, FibEvaluator},
};
use common::stark::{Assertion, ConstraintEvaluator, IoAssertionEvaluator};
use crypto::hash::blake3;
use math::{fft, field};

#[test]
fn evaluate_constraints() {
    // evaluate constraints
    let trace_length = 8;
    let blowup_factor = 4;
    let evaluations = build_constraint_evaluations(trace_length, blowup_factor);

    // transition constraints must be evaluations of degree 15 polynomial
    assert_eq!(15, infer_degree(evaluations.transition_evaluations()));

    // boundary constraints must be evaluations of degree 9 polynomial
    assert_eq!(9, infer_degree(evaluations.input_evaluations()));
    assert_eq!(9, infer_degree(evaluations.output_evaluations()));

    // TODO: clean-up this test

    let stride = 2;

    // transition constraint evaluations must be all 0s, except for the last step
    for &evaluation in evaluations
        .transition_evaluations()
        .iter()
        .rev()
        .skip(stride)
        .rev()
        .step_by(stride)
    {
        assert_eq!(0, evaluation);
    }
    assert_ne!(
        0,
        evaluations.transition_evaluations()[(trace_length - 1) * stride]
    );

    // input assertion evaluations must be 0 only at the first step
    assert_eq!(0, evaluations.input_evaluations()[0]);
    for &evaluation in evaluations
        .input_evaluations()
        .iter()
        .skip(stride)
        .step_by(stride)
    {
        assert_ne!(0, evaluation);
    }

    // input assertion evaluations must be 0 only at the first step
    for &evaluation in evaluations
        .output_evaluations()
        .iter()
        .rev()
        .skip(stride)
        .rev()
        .step_by(stride)
    {
        assert_ne!(0, evaluation);
    }
    assert_eq!(0, evaluations.output_evaluations()[(trace_length - 1) * 2]);
}

#[test]
fn build_constraint_poly() {
    // evaluate constraints
    let trace_length = 8;
    let blowup_factor = 4;
    let evaluations = build_constraint_evaluations(trace_length, blowup_factor);

    let trace_info = TraceInfo::new(2, trace_length, blowup_factor);
    let constraint_poly = super::build_constraint_poly(evaluations, &trace_info);

    assert_eq!(8, constraint_poly.degree());
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_constraint_evaluations(
    trace_length: usize,
    blowup_factor: usize,
) -> ConstraintEvaluationTable {
    let domain_size = trace_length * blowup_factor;
    let trace = build_trace(trace_length);
    let result = trace.get(1, trace_length - 1);
    let lde_root = field::get_root_of_unity(domain_size);
    let lde_domain = field::get_power_series(lde_root, domain_size);
    let lde_twiddles = fft::get_twiddles(lde_root, domain_size);
    let (extended_trace, _) = extend_trace(trace, &lde_twiddles);

    // commit to the trace
    let trace_tree = commit_trace(&extended_trace, blake3);

    // build constraint evaluator
    let trace_info = TraceInfo::new(2, trace_length, blowup_factor);
    let assertions = vec![
        Assertion::new(0, 0, 1),
        Assertion::new(1, 0, 1),
        Assertion::new(1, trace_length - 1, result),
    ];
    let evaluator = ConstraintEvaluator::<FibEvaluator, IoAssertionEvaluator>::new(
        *trace_tree.root(),
        &trace_info,
        &assertions,
    );

    // evaluate constraints
    super::evaluate_constraints(&evaluator, &extended_trace, &lde_domain)
}

fn build_trace(length: usize) -> super::TraceTable {
    let trace = build_fib_trace(length * 2);
    super::TraceTable::new(trace)
}
