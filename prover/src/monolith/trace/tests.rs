use crate::tests::{build_fib_trace, build_proof_context};
use crypto::{hash::blake3, MerkleTree};
use math::{
    field::{AsBytes, FieldElement, StarkField},
    polynom,
};

#[test]
fn new_trace_table() {
    let trace_length = 8;
    let trace = super::TraceTable::new(build_fib_trace(trace_length * 2));

    assert_eq!(2, trace.num_registers());
    assert_eq!(8, trace.num_states());

    let expected: Vec<FieldElement> = vec![1u32, 2, 5, 13, 34, 89, 233, 610]
        .into_iter()
        .map(FieldElement::from)
        .collect();
    assert_eq!(expected, trace.get_register(0));

    let expected: Vec<FieldElement> = vec![1u32, 3, 8, 21, 55, 144, 377, 987]
        .into_iter()
        .map(FieldElement::from)
        .collect();
    assert_eq!(expected, trace.get_register(1));
}

#[test]
fn extend_trace_table() {
    // build and extend trace table
    let trace_length = 8;
    let context = build_proof_context(trace_length, 2, 4);
    let trace = super::TraceTable::new(build_fib_trace(trace_length * 2));
    let lde_domain = super::build_lde_domain(&context);
    let (trace, trace_polys) = super::extend_trace(trace, &lde_domain);

    assert_eq!(2, trace.num_registers());
    assert_eq!(32, trace.num_states());

    // make sure trace polynomials evaluate to Fibonacci trace
    let trace_root = FieldElement::get_root_of_unity(trace_length.trailing_zeros());
    let trace_domain = FieldElement::get_power_series(trace_root, trace_length);
    assert_eq!(2, trace_polys.num_polys());
    assert_eq!(
        vec![1u32, 2, 5, 13, 34, 89, 233, 610]
            .into_iter()
            .map(FieldElement::from)
            .collect::<Vec<FieldElement>>(),
        polynom::eval_many(trace_polys.get_poly(0), &trace_domain)
    );
    assert_eq!(
        vec![1u32, 3, 8, 21, 55, 144, 377, 987]
            .into_iter()
            .map(FieldElement::from)
            .collect::<Vec<FieldElement>>(),
        polynom::eval_many(trace_polys.get_poly(1), &trace_domain)
    );

    // make sure register values are consistent with trace polynomials
    assert_eq!(
        trace_polys.get_poly(0),
        polynom::interpolate(&lde_domain.values(), trace.get_register(0), true)
    );
    assert_eq!(
        trace_polys.get_poly(1),
        polynom::interpolate(&lde_domain.values(), trace.get_register(1), true)
    );
}

#[test]
fn commit_trace_table() {
    // build and extend trace table
    let trace_length = 8;
    let context = build_proof_context(trace_length, 2, 4);
    let trace = super::TraceTable::new(build_fib_trace(trace_length * 2));
    let lde_domain = super::build_lde_domain(&context);
    let (trace, _) = super::extend_trace(trace, &lde_domain);

    // commit to the trace
    let trace_tree = super::build_trace_tree(&trace, blake3);

    // build Merkle tree from trace rows
    let mut hashed_states = Vec::new();
    let mut trace_state = vec![FieldElement::ZERO; trace.num_registers()];
    #[allow(clippy::needless_range_loop)]
    for i in 0..trace.num_states() {
        for j in 0..trace.num_registers() {
            trace_state[j] = trace.get(j, i);
        }
        let mut buf = [0; 32];
        blake3(trace_state.as_slice().as_bytes(), &mut buf);
        hashed_states.push(buf);
    }
    let expected_tree = MerkleTree::new(hashed_states, blake3);

    // compare the result
    assert_eq!(expected_tree.root(), trace_tree.root())
}
