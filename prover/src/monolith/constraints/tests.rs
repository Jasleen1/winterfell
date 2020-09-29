use super::{utils::infer_degree, ConstraintEvaluationTable};
use crate::{
    channel::ProverChannel,
    monolith::{build_trace_tree, extend_trace},
    tests::{build_fib_trace, FibEvaluator},
};
use common::stark::{
    Assertion, ConstraintEvaluator, IoAssertionEvaluator, ProofContext, ProofOptions,
};
use crypto::hash::blake3;

#[test]
fn evaluate_constraints() {
    // evaluate constraints
    let trace_length = 8;
    let blowup_factor = 4;
    let evaluations = build_constraint_evaluations(trace_length, blowup_factor).into_vec();
    let transition_evaluations = &evaluations[0];
    let input_evaluations = &evaluations[1];
    let output_evaluations = &evaluations[2];

    // transition constraints must be evaluations of degree 15 polynomial
    assert_eq!(15, infer_degree(transition_evaluations));

    // boundary constraints must be evaluations of degree 9 polynomial
    assert_eq!(9, infer_degree(input_evaluations));
    assert_eq!(9, infer_degree(output_evaluations));

    // TODO: clean-up this test

    let stride = 2;

    // transition constraint evaluations must be all 0s, except for the last step
    for &evaluation in transition_evaluations
        .iter()
        .rev()
        .skip(stride)
        .rev()
        .step_by(stride)
    {
        assert_eq!(0, evaluation);
    }
    assert_ne!(0, transition_evaluations[(trace_length - 1) * stride]);

    // input assertion evaluations must be 0 only at the first step
    assert_eq!(0, input_evaluations[0]);
    for &evaluation in input_evaluations.iter().skip(stride).step_by(stride) {
        assert_ne!(0, evaluation);
    }

    // input assertion evaluations must be 0 only at the first step
    for &evaluation in output_evaluations
        .iter()
        .rev()
        .skip(stride)
        .rev()
        .step_by(stride)
    {
        assert_ne!(0, evaluation);
    }
    assert_eq!(0, output_evaluations[(trace_length - 1) * 2]);
}

#[test]
fn build_constraint_poly() {
    // evaluate constraints
    let trace_length = 8;
    let blowup_factor = 4;
    let context = build_proof_context(trace_length, blowup_factor);
    let evaluations = build_constraint_evaluations(trace_length, blowup_factor);

    let constraint_poly = super::build_constraint_poly(evaluations, &context);

    assert_eq!(8, constraint_poly.degree());
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_constraint_evaluations(
    trace_length: usize,
    blowup_factor: usize,
) -> ConstraintEvaluationTable {
    // build proof context
    let context = build_proof_context(trace_length, blowup_factor);

    let trace = build_trace(trace_length);
    let result = trace.get(1, trace_length - 1);
    let lde_domain = super::super::build_lde_domain(&context);
    let (extended_trace, _) = extend_trace(trace, &lde_domain);

    // commit to the trace
    let mut channel = ProverChannel::new(&context);
    let trace_tree = build_trace_tree(&extended_trace, blake3);
    channel.commit_trace(*trace_tree.root());

    // build constraint evaluator
    let assertions = vec![
        Assertion::new(0, 0, 1),
        Assertion::new(1, 0, 1),
        Assertion::new(1, trace_length - 1, result),
    ];
    let mut evaluator = ConstraintEvaluator::<FibEvaluator, IoAssertionEvaluator>::new(
        &channel, &context, assertions,
    );

    // evaluate constraints
    super::evaluate_constraints(&mut evaluator, &extended_trace, &lde_domain)
}

fn build_trace(length: usize) -> super::TraceTable {
    let trace = build_fib_trace(length * 2);
    super::TraceTable::new(trace)
}

fn build_proof_context(trace_length: usize, blowup: usize) -> ProofContext {
    let options = ProofOptions::new(32, blowup, 0, blake3);
    ProofContext::new(2, trace_length, 1, options)
}
