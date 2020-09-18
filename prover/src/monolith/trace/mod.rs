use super::types::{PolyTable, TraceTable};
use common::{
    stark::TraceInfo,
    utils::{as_bytes, uninit_vector},
};
use crypto::{HashFunction, MerkleTree};
use math::{fft, field};

#[cfg(test)]
mod tests;

// PROCEDURES
// ================================================================================================

/// Builds and return evaluation domain and twiddles for STARK proof. Twiddles
// are used in FFT evaluation and are half the size of evaluation domain.
pub fn build_lde_domain(trace_info: &TraceInfo) -> (Vec<u128>, Vec<u128>) {
    let root = field::get_root_of_unity(trace_info.lde_domain_size());
    let domain = field::get_power_series(root, trace_info.lde_domain_size());

    // it is more efficient to build by taking half of the domain and permuting it, rather than
    // building twiddles from scratch using fft::get_twiddles()
    let mut twiddles = domain[..(domain.len() / 2)].to_vec();
    fft::permute(&mut twiddles);

    (domain, twiddles)
}

/// Extends all registers of the trace table to the length of the evaluation domain;
/// The extension is done by first interpolating a register into a polynomial and then
/// evaluating the polynomial over the evaluation domain.
pub fn extend_trace(trace: TraceTable, lde_twiddles: &[u128]) -> (TraceTable, PolyTable) {
    // evaluation domain size is twice the number of twiddles
    let domain_size = lde_twiddles.len() * 2;
    let trace_length = trace.num_states();
    assert!(
        domain_size > trace_length,
        "evaluation domain must be larger than execution trace length"
    );

    // build trace twiddles for FFT interpolation over trace domain
    let trace_root = field::get_root_of_unity(trace_length);
    let trace_twiddles = fft::get_inv_twiddles(trace_root, trace_length);

    let mut polys = trace.into_vec();
    let mut trace = Vec::new();

    // extend all registers
    for poly in polys.iter_mut() {
        // interpolate register trace into a polynomial
        fft::interpolate_poly(poly, &trace_twiddles, true);

        // allocate space to hold extended evaluations and copy the polynomial into it
        let mut register = vec![field::ZERO; domain_size];
        register[..poly.len()].copy_from_slice(&poly);

        // evaluate the polynomial over extended domain
        fft::evaluate_poly(&mut register, &lde_twiddles, true);
        trace.push(register);
    }

    (TraceTable::new(trace), PolyTable::new(polys))
}

/// Builds a Merkle tree out of trace table rows (hash of each row becomes a leaf in the tree).
pub fn commit_trace(trace: &TraceTable, hash: HashFunction) -> MerkleTree {
    // allocate vector to store row hashes
    let mut hashed_states = uninit_vector::<[u8; 32]>(trace.num_states());

    // iterate though table rows, hashing each row
    let mut trace_state = vec![field::ZERO; trace.num_registers()];
    #[allow(clippy::needless_range_loop)]
    for i in 0..trace.num_states() {
        trace.copy_row(i, &mut trace_state);
        hash(as_bytes(&trace_state), &mut hashed_states[i]);
    }

    // build Merkle tree out of hashed rows
    MerkleTree::new(hashed_states, hash)
}

pub fn query_trace(_trace: TraceTable, _trace_tree: MerkleTree, _positions: Vec<usize>) {
    // TODO
}
