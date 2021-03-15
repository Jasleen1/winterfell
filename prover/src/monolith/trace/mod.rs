use super::types::{LdeDomain, PolyTable, TraceTable};
use common::utils::uninit_vector;
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::{
    fft,
    field::{AsBytes, BaseElement, FieldElement},
};

#[cfg(test)]
mod tests;

// PROCEDURES
// ================================================================================================

/// Extends all registers of the trace table to the length of the evaluation domain;
/// The extension is done by first interpolating a register into a polynomial and then
/// evaluating the polynomial over the evaluation domain.
pub fn extend_trace(trace: TraceTable, lde_domain: &LdeDomain) -> (TraceTable, PolyTable) {
    let trace_length = trace.num_states();
    assert!(
        lde_domain.size() > trace_length,
        "evaluation domain must be larger than execution trace length"
    );

    // build trace twiddles for FFT interpolation over trace domain
    let trace_twiddles = fft::get_inv_twiddles::<BaseElement>(trace_length);

    let mut polys = trace.into_vec();
    let mut trace = Vec::new();

    // extend all registers
    for poly in polys.iter_mut() {
        // interpolate register trace into a polynomial
        fft::interpolate_poly(poly, &trace_twiddles);

        // allocate space to hold extended evaluations and copy the polynomial into it
        let mut register = vec![BaseElement::ZERO; lde_domain.size()];
        register[..poly.len()].copy_from_slice(&poly);

        // evaluate the polynomial over extended domain
        fft::evaluate_poly(&mut register, &lde_domain.twiddles());
        trace.push(register);
    }

    (TraceTable::new(trace), PolyTable::new(polys))
}

/// Builds a Merkle tree out of trace table rows (hash of each row becomes a leaf in the tree).
pub fn build_trace_tree(trace: &TraceTable, hash: HashFunction) -> MerkleTree {
    // allocate vector to store row hashes
    let mut hashed_states = uninit_vector::<[u8; 32]>(trace.num_states());

    // iterate though table rows, hashing each row
    let mut trace_state = vec![BaseElement::ZERO; trace.num_registers()];
    #[allow(clippy::needless_range_loop)]
    for i in 0..trace.num_states() {
        trace.copy_row(i, &mut trace_state);
        hash(trace_state.as_slice().as_bytes(), &mut hashed_states[i]);
    }

    // build Merkle tree out of hashed rows
    MerkleTree::new(hashed_states, hash)
}

/// Returns trace table rows at the specified positions along with Merkle
/// authentication paths from the trace_tree root to these rows.
pub fn query_trace(
    trace: TraceTable,
    trace_tree: MerkleTree,
    positions: &[usize],
) -> (BatchMerkleProof, Vec<Vec<BaseElement>>) {
    // allocate memory for queried trace states
    let mut trace_states = Vec::with_capacity(positions.len());

    // copy values from the trace table at the specified positions into rows
    // and append the rows to trace_states
    let trace = trace.into_vec();
    for &i in positions.iter() {
        let row = trace.iter().map(|r| r[i]).collect();
        trace_states.push(row);
    }

    // build Merkle authentication paths to the leaves specified by positions
    let trace_proof = trace_tree.prove_batch(&positions);

    (trace_proof, trace_states)
}
