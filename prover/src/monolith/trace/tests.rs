use common::utils::as_bytes;
use crypto::{hash::blake3, MerkleTree};
use math::{fft, field, polynom};

#[test]
fn new_trace_table() {
    let trace = build_trace(8);

    assert_eq!(2, trace.num_registers());
    assert_eq!(8, trace.num_states());

    assert_eq!(vec![1, 2, 5, 13, 34, 89, 233, 610], trace.get_register(0));
    assert_eq!(vec![1, 3, 8, 21, 55, 144, 377, 987], trace.get_register(1));
}

#[test]
fn extend_trace_table() {
    // build and extend trace table
    let trace_length = 8;
    let domain_size = trace_length * 4;
    let trace = build_trace(8);
    let lde_root = field::get_root_of_unity(domain_size);
    let lde_twiddles = fft::get_twiddles(lde_root, domain_size);
    let (trace, trace_polys) = super::extend_trace(trace, &lde_twiddles);

    assert_eq!(2, trace.num_registers());
    assert_eq!(32, trace.num_states());

    // make sure trace polynomials evaluate to Fibonacci trace
    let trace_root = field::get_root_of_unity(trace_length);
    let trace_domain = field::get_power_series(trace_root, trace_length);
    assert_eq!(2, trace_polys.num_polys());
    assert_eq!(
        vec![1, 2, 5, 13, 34, 89, 233, 610],
        polynom::eval_many(trace_polys.get_poly(0), &trace_domain)
    );
    assert_eq!(
        vec![1, 3, 8, 21, 55, 144, 377, 987],
        polynom::eval_many(trace_polys.get_poly(1), &trace_domain)
    );

    // make sure register values are consistent with trace polynomials
    let lde_domain = field::get_power_series(lde_root, domain_size);
    assert_eq!(
        trace_polys.get_poly(0),
        polynom::interpolate(&lde_domain, trace.get_register(0), true)
    );
    assert_eq!(
        trace_polys.get_poly(1),
        polynom::interpolate(&lde_domain, trace.get_register(1), true)
    );
}

#[test]
fn commit_trace_table() {
    // build and extend trace table
    let trace_length = 8;
    let domain_size = trace_length * 4;
    let trace = build_trace(8);
    let lde_root = field::get_root_of_unity(domain_size);
    let lde_twiddles = fft::get_twiddles(lde_root, domain_size);
    let (trace, _) = super::extend_trace(trace, &lde_twiddles);

    // commit to the trace
    let trace_tree = super::commit_trace(&trace, blake3);

    // build Merkle tree from trace rows
    let mut hashed_states = Vec::new();
    let mut trace_state = vec![field::ZERO; trace.num_registers()];
    #[allow(clippy::needless_range_loop)]
    for i in 0..trace.num_states() {
        for j in 0..trace.num_registers() {
            trace_state[j] = trace.get(j, i);
        }
        let mut buf = [0; 32];
        blake3(as_bytes(&trace_state), &mut buf);
        hashed_states.push(buf);
    }
    let expected_tree = MerkleTree::new(hashed_states, blake3);

    // compare the result
    assert_eq!(expected_tree.root(), trace_tree.root())
}

fn build_trace(length: usize) -> super::TraceTable {
    let trace = crate::tests::build_fib_trace(length * 2);
    super::TraceTable::new(trace)
}
