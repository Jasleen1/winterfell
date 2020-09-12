use crypto::hash::blake3;
use math::{fft, field, polynom};

#[test]
fn new_trace_table() {
    let trace = build_trace(8, 4);

    assert_eq!(2, trace.register_count());
    assert_eq!(8, trace.unextended_length());
    assert_eq!(32, trace.domain_size());
    assert_eq!(4, trace.blowup_factor());
    assert_eq!(false, trace.is_extended());
    assert_eq!(false, trace.is_committed());

    assert_eq!(vec![1, 2, 5, 13, 34, 89, 233, 610], trace.registers[0]);
    assert_eq!(vec![1, 3, 8, 21, 55, 144, 377, 987], trace.registers[1]);

    assert_eq!(0, trace.polys.len());
}

#[test]
fn extend_trace_table() {
    // build and extend trace table
    let mut trace = build_trace(8, 4);
    let lde_root = field::get_root_of_unity(trace.domain_size());
    let lde_twiddles = fft::get_twiddles(lde_root, trace.domain_size());
    trace.extend(&lde_twiddles);

    assert_eq!(2, trace.register_count());
    assert_eq!(8, trace.unextended_length());
    assert_eq!(32, trace.domain_size());
    assert_eq!(4, trace.blowup_factor());
    assert_eq!(true, trace.is_extended());
    assert_eq!(false, trace.is_committed());

    // make sure trace polynomials evaluate to Fibonacci trace
    let trace_root = field::get_root_of_unity(trace.unextended_length());
    let trace_domain = field::get_power_series(trace_root, trace.unextended_length());
    assert_eq!(2, trace.polys.len());
    assert_eq!(
        vec![1, 2, 5, 13, 34, 89, 233, 610],
        polynom::eval_many(&trace.polys[0], &trace_domain)
    );
    assert_eq!(
        vec![1, 3, 8, 21, 55, 144, 377, 987],
        polynom::eval_many(&trace.polys[1], &trace_domain)
    );

    // make sure register values are consistent with trace polynomials
    let lde_domain = field::get_power_series(lde_root, trace.domain_size());
    assert_eq!(
        trace.polys[0],
        polynom::interpolate(&lde_domain, &trace.registers[0], true)
    );
    assert_eq!(
        trace.polys[1],
        polynom::interpolate(&lde_domain, &trace.registers[1], true)
    );
}

#[test]
fn commit_trace_table() {
    // build and extend trace table
    let mut trace = build_trace(8, 4);
    let lde_root = field::get_root_of_unity(trace.domain_size());
    let lde_twiddles = fft::get_twiddles(lde_root, trace.domain_size());
    trace.extend(&lde_twiddles);

    // commit to the trace
    let commitment = trace.commit(blake3);

    assert_eq!(2, trace.register_count());
    assert_eq!(8, trace.unextended_length());
    assert_eq!(32, trace.domain_size());
    assert_eq!(4, trace.blowup_factor());
    assert_eq!(true, trace.is_extended());
    assert_eq!(true, trace.is_committed());

    assert_eq!(commitment, *trace.trace_tree.unwrap().root());
}

fn build_trace(length: usize, blowup: usize) -> super::TraceTable {
    let trace = crate::utils::build_fib_trace(length * 2);
    let options = crate::ProofOptions::new(40, blowup, 10, blake3);
    super::TraceTable::new(trace, &options)
}
