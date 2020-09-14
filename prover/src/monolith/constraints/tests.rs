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
    let lde_twiddles = fft::get_twiddles(lde_root, domain_size);
    let (trace, _) = extend_trace(trace, &lde_twiddles);

    // commit to the trace
    let trace_tree = commit_trace(&trace, blake3);

    let trace_info = TraceInfo::new(2, trace_length, blowup_factor);
    let assertions = vec![Assertion::new(1, 7, 987)];
    let evaluator = ConstraintEvaluator::<FibEvaluator, IoAssertionEvaluator>::new(
        *trace_tree.root(),
        trace_info,
        &assertions,
    );

    let mut state1 = vec![0; trace.num_registers()];
    let mut state2 = vec![0; trace.num_registers()];

    trace.copy_row(0, &mut state1);
    trace.copy_row(4, &mut state2);

    let evaluations = evaluator.evaluate(&state1, &state2, field::exp(lde_root, 0), 0);
    assert_eq!(0, evaluations.0);
}

fn build_trace(length: usize) -> super::TraceTable {
    let trace = crate::utils::build_fib_trace(length * 2);
    super::TraceTable::new(trace)
}
