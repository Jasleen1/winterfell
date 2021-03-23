use super::{types::PolyTable, StarkDomain};
use common::utils::uninit_vector;
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::{
    fft,
    field::{AsBytes, BaseElement, FieldElement},
};
use std::mem;

#[cfg(feature = "concurrent")]
use rayon::prelude::*;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

const MIN_TRACE_LENGTH: usize = 8;

// TRACE TABLE
// ================================================================================================
pub struct TraceTable(Vec<Vec<BaseElement>>);

impl TraceTable {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new trace table from a list of provided register traces.
    pub fn new(registers: Vec<Vec<BaseElement>>) -> Self {
        assert!(
            !registers.is_empty(),
            "execution trace must consist of at least one register"
        );
        let trace_length = registers[0].len();
        assert!(
            trace_length >= MIN_TRACE_LENGTH,
            "execution trace must be at lest {} steps long, but was {}",
            MIN_TRACE_LENGTH,
            trace_length
        );
        assert!(
            trace_length.is_power_of_two(),
            "execution trace length must be a power of 2"
        );
        for register in registers.iter() {
            assert!(
                register.len() == trace_length,
                "all register traces must have the same length"
            );
        }

        TraceTable(registers)
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns number of registers in the trace table.
    pub fn num_registers(&self) -> usize {
        self.0.len()
    }

    /// Returns the number of states in this trace table.
    pub fn num_states(&self) -> usize {
        self.0[0].len()
    }

    /// Returns value in the specified `register` at the specified `step`.
    pub fn get(&self, register: usize, step: usize) -> BaseElement {
        self.0[register][step]
    }

    /// Returns the entire register trace for the register at the specified index.
    #[cfg(test)]
    pub fn get_register(&self, idx: usize) -> &[BaseElement] {
        &self.0[idx]
    }

    /// Copies values of all registers at the specified `step` into the `destination` slice.
    pub fn copy_row(&self, step: usize, destination: &mut [BaseElement]) {
        for (i, register) in self.0.iter().enumerate() {
            destination[i] = register[step];
        }
    }

    // LOW-DEGREE EXTENSION
    // --------------------------------------------------------------------------------------------
    /// Extends all registers of the trace table to the length of the LDE domain; The extension
    /// is done by first interpolating a register into a polynomial and then evaluating the
    /// polynomial over the LDE domain.
    pub fn extend(&mut self, domain: &StarkDomain) -> PolyTable {
        assert_eq!(
            self.num_states(),
            domain.trace_length(),
            "inconsistent trace length"
        );
        // build and cache trace twiddles for FFT interpolation; we do it here so that we
        // don't have to rebuild these twiddles for every register.
        let inv_twiddles = fft::get_inv_twiddles::<BaseElement>(domain.trace_length());

        // extend all registers (either in multiple threads or in a single thread); the extension
        // procedure first interpolates register traces into polynomials (in-place), then evaluates
        // these polynomials over a larger domain, and then returns extended evaluations.
        #[cfg(feature = "concurrent")]
        let mut extended_trace = self
            .0
            .par_iter_mut()
            .map(|register_trace| extend_register(register_trace, &domain, &inv_twiddles))
            .collect();

        #[cfg(not(feature = "concurrent"))]
        let mut extended_trace = self
            .0
            .iter_mut()
            .map(|register_trace| extend_register(register_trace, &domain, &inv_twiddles))
            .collect();

        // keep the extended trace in this struct, and return trace polynomials
        mem::swap(&mut extended_trace, &mut self.0);
        PolyTable::new(extended_trace)
    }

    // TRACE COMMITMENT
    // --------------------------------------------------------------------------------------------
    /// Builds a Merkle tree out of trace table rows (hash of each row becomes a leaf in the tree).
    pub fn build_commitment(&self, hash: HashFunction) -> MerkleTree {
        // allocate vector to store row hashes
        let mut hashed_states = uninit_vector::<[u8; 32]>(self.num_states());

        // iterate though table rows, hashing each row; the hashing is done by first copying
        // the state into trace_state buffer to avoid unneeded allocations, and then by applying
        // the hash function to the buffer.
        #[cfg(feature = "concurrent")]
        {
            let batch_size = hashed_states.len() / rayon::current_num_threads().next_power_of_two();
            hashed_states
                .par_chunks_mut(batch_size)
                .enumerate()
                .for_each(|(batch_idx, hashed_states_batch)| {
                    let offset = batch_idx * batch_size;
                    let mut trace_state = vec![BaseElement::ZERO; self.num_registers()];
                    for (i, row_hash) in hashed_states_batch.iter_mut().enumerate() {
                        self.copy_row(i + offset, &mut trace_state);
                        hash(trace_state.as_slice().as_bytes(), row_hash);
                    }
                });
        }

        #[cfg(not(feature = "concurrent"))]
        {
            let mut trace_state = vec![BaseElement::ZERO; self.num_registers()];
            for (i, row_hash) in hashed_states.iter_mut().enumerate() {
                self.copy_row(i, &mut trace_state);
                hash(trace_state.as_slice().as_bytes(), row_hash);
            }
        }

        // build Merkle tree out of hashed rows
        MerkleTree::new(hashed_states, hash)
    }

    // QUERY TRACE
    // --------------------------------------------------------------------------------------------
    /// Returns trace table rows at the specified positions along with Merkle authentication paths
    /// from the `commitment` root to these rows.
    pub fn query(
        &self,
        commitment: MerkleTree,
        positions: &[usize],
    ) -> (BatchMerkleProof, Vec<Vec<BaseElement>>) {
        assert_eq!(
            self.num_states(),
            commitment.leaves().len(),
            "inconsistent trace table commitment"
        );

        // allocate memory for queried trace states
        let mut trace_states = Vec::with_capacity(positions.len());

        // copy values from the trace table at the specified positions into rows
        // and append the rows to trace_states
        for &i in positions.iter() {
            let row = self.0.iter().map(|r| r[i]).collect();
            trace_states.push(row);
        }

        // build Merkle authentication paths to the leaves specified by positions
        let trace_proof = commitment.prove_batch(&positions);

        (trace_proof, trace_states)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

#[inline(always)]
fn extend_register(
    trace: &mut [BaseElement],
    domain: &StarkDomain,
    inv_twiddles: &[BaseElement],
) -> Vec<BaseElement> {
    let domain_offset = domain.offset();
    let twiddles = domain.trace_twiddles();
    let blowup_factor = domain.trace_to_lde_blowup();

    // interpolate register trace into a polynomial
    fft::interpolate_poly_with_offset(trace, inv_twiddles, domain_offset);

    // evaluate the polynomial over extended domain
    fft::evaluate_poly_with_offset(trace, twiddles, domain_offset, blowup_factor)
}
