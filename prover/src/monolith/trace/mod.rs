use super::types::{LdeDomain, PolyTable, TraceTable};
use common::{
    stark::ProofContext,
    utils::{as_bytes, uninit_vector},
};
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::{
    fft,
    field::{f128::FieldElement, StarkField},
};

#[cfg(test)]
mod tests;

// PROCEDURES
// ================================================================================================

/// Builds and return evaluation domain for STARK proof.
pub fn build_lde_domain(context: &ProofContext) -> LdeDomain {
    let domain =
        FieldElement::get_power_series(context.generators().lde_domain, context.lde_domain_size());

    // it is more efficient to build by taking half of the domain and permuting it, rather than
    // building twiddles from scratch using fft::get_twiddles()
    let mut twiddles = domain[..(domain.len() / 2)].to_vec();
    fft::permute(&mut twiddles);

    LdeDomain::new(domain, twiddles)
}

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
    let trace_root = FieldElement::get_root_of_unity(trace_length.trailing_zeros() as usize);
    let trace_twiddles = fft::get_inv_twiddles(trace_root, trace_length);

    let mut polys = trace.into_vec();
    let mut trace = Vec::new();

    // extend all registers
    for poly in polys.iter_mut() {
        // interpolate register trace into a polynomial
        fft::interpolate_poly(poly, &trace_twiddles, true);

        // allocate space to hold extended evaluations and copy the polynomial into it
        let mut register = vec![FieldElement::ZERO; lde_domain.size()];
        register[..poly.len()].copy_from_slice(&poly);

        // evaluate the polynomial over extended domain
        fft::evaluate_poly(&mut register, &lde_domain.twiddles(), true);
        trace.push(register);
    }

    (TraceTable::new(trace), PolyTable::new(polys))
}

/// Builds a Merkle tree out of trace table rows (hash of each row becomes a leaf in the tree).
pub fn build_trace_tree(trace: &TraceTable, hash: HashFunction) -> MerkleTree {
    // allocate vector to store row hashes
    let mut hashed_states = uninit_vector::<[u8; 32]>(trace.num_states());

    // iterate though table rows, hashing each row
    let mut trace_state = vec![FieldElement::ZERO; trace.num_registers()];
    #[allow(clippy::needless_range_loop)]
    for i in 0..trace.num_states() {
        trace.copy_row(i, &mut trace_state);
        hash(as_bytes(&trace_state), &mut hashed_states[i]);
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
) -> (BatchMerkleProof, Vec<Vec<FieldElement>>) {
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
