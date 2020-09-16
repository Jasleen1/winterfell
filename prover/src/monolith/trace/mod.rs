use super::types::{PolyTable, TraceTable};
use super::utils;
use crate::TraceInfo;
use common::utils::{as_bytes, uninit_vector};
use crypto::{HashFunction, MerkleTree};
use math::{fft, field, polynom};

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

/// Combines trace polynomials for all registers into a single composition polynomial.
/// The combination is done as follows:
/// 1. First, state of trace registers at deep points z and z * g are computed;
/// 2. Then, polynomials T1_i(x) = (T_i(x) - T_i(z)) / (x - z) and 
/// T2_i(x) = (T_i(x) - T_i(z * g)) / (x - z * g) are computed for all i and combined
/// together into a single polynomial using a pseudo-random linear combination;
/// 3. Then the degree of the polynomial is adjusted to match the composition degree
pub fn combine_trace_polys(
    polys: PolyTable,
    z: u128,
    cc: &CompositionCoefficients,
) -> (Vec<u128>, Vec<u128>, Vec<u128>) {
    let trace_length = polys.poly_size();

    let g = field::get_root_of_unity(trace_length);
    let next_z = field::mul(z, g);

    // compute state of registers at deep points z and z * g
    let trace_state1 = polys.evaluate_at(z);
    let trace_state2 = polys.evaluate_at(next_z);

    let mut t1_composition = vec![field::ZERO; trace_length];
    let mut t2_composition = vec![field::ZERO; trace_length];

    // combine trace polynomials into 2 composition polynomials T1(x) and T2(x)
    let polys = polys.into_vec();
    for i in 0..polys.len() {
        // compute T1(x) = (T(x) - T(z)), multiply it by a pseudo-random coefficient,
        // and add the result into composition polynomial
        utils::mul_acc(&mut t1_composition, &polys[i], cc.trace1[i]);
        let adjusted_tz = field::mul(trace_state1[i], cc.trace1[i]);
        t1_composition[0] = field::sub(t1_composition[0], adjusted_tz);

        // compute T2(x) = (T(x) - T(z * g)), multiply it by a pseudo-random
        // coefficient, and add the result into composition polynomial
        utils::mul_acc(&mut t2_composition, &polys[i], cc.trace2[i]);
        let adjusted_tz = field::mul(trace_state2[i], cc.trace2[i]);
        t2_composition[0] = field::sub(t2_composition[0], adjusted_tz);
    }

    // divide the two composition polynomials by (x - z) and (x - z * g)
    // respectively and add the resulting polynomials together
    polynom::syn_div_in_place(&mut t1_composition, z);
    polynom::syn_div_in_place(&mut t2_composition, next_z);
    utils::add_in_place(&mut t1_composition, &t2_composition);

    // adjust the degree of the polynomial to match the degree parameter by computing
    // C(x) = T(x) * k_1 + T(x) * x^incremental_degree * k_2
    let poly_size = 0; // TODO: utils::get_composition_degree(trace_length).next_power_of_two();
    let mut composition_poly = vec![0; poly_size]; // TODO: filled_vector(poly_size, self.domain_size(), field::ZERO);
    let incremental_degree = 0; // TODO: utils::get_incremental_trace_degree(trace_length);
                                // this is equivalent to T(x) * k_1
    utils::mul_acc(
        &mut composition_poly[..trace_length],
        &t1_composition,
        cc.t1_degree,
    );
    // this is equivalent to T(x) * x^incremental_degree * k_2
    utils::mul_acc(
        &mut composition_poly[incremental_degree..(incremental_degree + trace_length)],
        &t1_composition,
        cc.t2_degree,
    );

    (composition_poly, trace_state1, trace_state2)
}

pub fn query_trace(_trace: TraceTable, _trace_tree: MerkleTree, _positions: Vec<usize>) {
    // TODO
}

// TODO: move to a better location
pub struct CompositionCoefficients {
    pub trace1: Vec<u128>,
    pub trace2: Vec<u128>,
    pub t1_degree: u128,
    pub t2_degree: u128,
    pub constraints: u128,
}
