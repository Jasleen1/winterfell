use crate::{
    evaluator::FibEvaluator,
    monolith::{commit_trace, extend_trace},
    Assertion, ConstraintEvaluator, IoAssertionEvaluator, TraceInfo,
};
use crypto::hash::blake3;
use math::{fft, field};

#[test]
fn evaluate_constraints() {
    // build and extend trace table
    let trace_length = 8;
    let blowup_factor = 4;
    let domain_size = trace_length * blowup_factor;
    let trace = build_trace(8);
    let lde_root = field::get_root_of_unity(domain_size);
    let lde_domain = field::get_power_series(lde_root, domain_size);
    let lde_twiddles = fft::get_twiddles(lde_root, domain_size);
    let (trace, _) = extend_trace(trace, &lde_twiddles);

    // commit to the trace
    let trace_tree = commit_trace(&trace, blake3);

    // build constraint evaluator
    let trace_info = TraceInfo::new(2, trace_length, blowup_factor);
    let assertions = vec![
        Assertion::new(0, 0, 1),
        Assertion::new(1, 0, 1),
        Assertion::new(1, 7, 987),
    ];
    let evaluator = ConstraintEvaluator::<FibEvaluator, IoAssertionEvaluator>::new(
        *trace_tree.root(),
        trace_info,
        &assertions,
    );

    // evaluate constraints
    let evaluations = super::evaluate_constraints(&evaluator, &trace, &lde_domain);

    // all constraint evaluations must represent polynomial with degree = deg(trace)
    assert_eq!(
        trace_length - 1,
        infer_degree(evaluations.transition_evaluations())
    );
    assert_eq!(
        trace_length - 1,
        infer_degree(evaluations.input_evaluations())
    );
    assert_eq!(
        trace_length - 1,
        infer_degree(evaluations.output_evaluations())
    );

    // transition constraint evaluations must be all 0s, except for the last step
    for &evaluation in evaluations.transition_evaluations().iter().rev().skip(1) {
        assert_eq!(0, evaluation);
    }
    assert_ne!(0, evaluations.transition_evaluations()[trace_length - 1]);

    // input assertion evaluations must be 0 only at the first step
    assert_eq!(0, evaluations.input_evaluations()[0]);
    for &evaluation in evaluations.input_evaluations().iter().skip(1) {
        assert_ne!(0, evaluation);
    }

    // input assertion evaluations must be 0 only at the first step
    for &evaluation in evaluations.output_evaluations().iter().rev().skip(1) {
        assert_ne!(0, evaluation);
    }
    assert_eq!(0, evaluations.output_evaluations()[trace_length - 1]);
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_trace(length: usize) -> super::TraceTable {
    let trace = crate::utils::build_fib_trace(length * 2);
    super::TraceTable::new(trace)
}

// TODO: move to utils
pub fn infer_degree(evaluations: &[u128]) -> usize {
    assert!(
        evaluations.len().is_power_of_two(),
        "number of evaluations must be a power of 2"
    );
    let mut poly = evaluations.to_vec();
    let root = field::get_root_of_unity(evaluations.len());
    let inv_twiddles = fft::get_inv_twiddles(root, evaluations.len());
    fft::interpolate_poly(&mut poly, &inv_twiddles, true);
    degree_of(&poly)
}

// TODO: move to utils
pub fn degree_of(poly: &[u128]) -> usize {
    for i in (0..poly.len()).rev() {
        if poly[i] != field::ZERO {
            return i;
        }
    }
    0
}
