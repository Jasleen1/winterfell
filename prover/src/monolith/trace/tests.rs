use crate::{
    monolith::StarkDomain,
    tests::{build_fib_trace, build_proof_context},
};
use crypto::{hash::blake3, MerkleTree};
use math::{
    field::{AsBytes, BaseElement, FieldElement, StarkField},
    polynom,
};

#[test]
fn new_trace_table() {
    let trace_length = 8;
    let trace = build_fib_trace(trace_length * 2);

    assert_eq!(2, trace.width());
    assert_eq!(8, trace.len());

    let expected: Vec<BaseElement> = vec![1u32, 2, 5, 13, 34, 89, 233, 610]
        .into_iter()
        .map(BaseElement::from)
        .collect();
    assert_eq!(expected, trace.get_register(0));

    let expected: Vec<BaseElement> = vec![1u32, 3, 8, 21, 55, 144, 377, 987]
        .into_iter()
        .map(BaseElement::from)
        .collect();
    assert_eq!(expected, trace.get_register(1));
}

#[test]
fn extend_trace_table() {
    // build and extend trace table
    let trace_length = 8;
    let context = build_proof_context(trace_length, 2, 4);
    let trace = build_fib_trace(trace_length * 2);
    let domain = StarkDomain::new(&context);
    let (extended_trace, trace_polys) = trace.extend(&domain);

    assert_eq!(2, extended_trace.width());
    assert_eq!(32, extended_trace.len());

    // make sure trace polynomials evaluate to Fibonacci trace
    let trace_root = BaseElement::get_root_of_unity(trace_length.trailing_zeros());
    let trace_domain = BaseElement::get_power_series(trace_root, trace_length);
    assert_eq!(2, trace_polys.num_polys());
    assert_eq!(
        vec![1u32, 2, 5, 13, 34, 89, 233, 610]
            .into_iter()
            .map(BaseElement::from)
            .collect::<Vec<BaseElement>>(),
        polynom::eval_many(trace_polys.get_poly(0), &trace_domain)
    );
    assert_eq!(
        vec![1u32, 3, 8, 21, 55, 144, 377, 987]
            .into_iter()
            .map(BaseElement::from)
            .collect::<Vec<BaseElement>>(),
        polynom::eval_many(trace_polys.get_poly(1), &trace_domain)
    );

    // make sure register values are consistent with trace polynomials
    assert_eq!(
        trace_polys.get_poly(0),
        polynom::interpolate(&domain.lde_values(), extended_trace.get_register(0), true)
    );
    assert_eq!(
        trace_polys.get_poly(1),
        polynom::interpolate(&domain.lde_values(), extended_trace.get_register(1), true)
    );
}

#[test]
fn commit_trace_table() {
    // build and extend trace table
    let trace_length = 8;
    let context = build_proof_context(trace_length, 2, 4);
    let trace = build_fib_trace(trace_length * 2);
    let domain = StarkDomain::new(&context);
    let (extended_trace, _) = trace.extend(&domain);

    // commit to the trace
    let trace_tree = extended_trace.build_commitment(blake3);

    // build Merkle tree from trace rows
    let mut hashed_states = Vec::new();
    let mut trace_state = vec![BaseElement::ZERO; extended_trace.width()];
    #[allow(clippy::needless_range_loop)]
    for i in 0..extended_trace.len() {
        for j in 0..extended_trace.width() {
            trace_state[j] = extended_trace.get(j, i);
        }
        let mut buf = [0; 32];
        blake3(trace_state.as_slice().as_bytes(), &mut buf);
        hashed_states.push(buf);
    }
    let expected_tree = MerkleTree::new(hashed_states, blake3);

    // compare the result
    assert_eq!(expected_tree.root(), trace_tree.root())
}
