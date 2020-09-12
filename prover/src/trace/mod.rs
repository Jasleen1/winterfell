use crate::ProofOptions;
use common::utils::{as_bytes, uninit_vector};
use crypto::{HashFunction, MerkleTree};
use math::{fft, field};

#[cfg(test)]
mod tests;

// TYPES AND INTERFACES
// ================================================================================================
pub struct TraceTable {
    registers: Vec<Vec<u128>>,
    polys: Vec<Vec<u128>>,
    trace_length: usize,
    blowup_factor: usize,
    trace_tree: Option<MerkleTree>,
}

// TRACE TABLE IMPLEMENTATION
// ================================================================================================
impl TraceTable {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a trace table constructed from the specified register traces.
    pub fn new(registers: Vec<Vec<u128>>, options: &ProofOptions) -> TraceTable {
        let trace_length = registers[0].len();
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

        let polys = Vec::with_capacity(registers.len());
        return TraceTable {
            registers,
            polys,
            trace_length,
            blowup_factor: options.blowup_factor(),
            trace_tree: None,
        };
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the number of registers in the trace table.
    pub fn register_count(&self) -> usize {
        return self.registers.len();
    }

    /// Returns the number of states in the un-extended trace table.
    pub fn unextended_length(&self) -> usize {
        return self.trace_length;
    }

    pub fn blowup_factor(&self) -> usize {
        return self.blowup_factor;
    }

    /// Returns the number of states in the extended trace table.
    pub fn domain_size(&self) -> usize {
        return self.trace_length * self.blowup_factor;
    }

    /// Returns `true` if the trace table has been extended.
    pub fn is_extended(&self) -> bool {
        return self.registers[0].len() > self.trace_length;
    }

    /// Returns `true` if the trace table commitment has been built.
    pub fn is_committed(&self) -> bool {
        return self.trace_tree.is_some();
    }

    // TRACE EXTENSION
    // --------------------------------------------------------------------------------------------
    /// Extends all registers of the trace table by the `blowup_factor` specified during
    /// trace table construction. A trace table can be extended only once.
    pub fn extend(&mut self, lde_twiddles: &[u128]) {
        assert!(!self.is_extended(), "trace table has already been extended");
        assert!(
            lde_twiddles.len() * 2 == self.domain_size(),
            "invalid number of twiddles"
        );

        // build trace twiddles for FFT interpolation over trace domain
        let trace_root = field::get_root_of_unity(self.unextended_length());
        let trace_twiddles = fft::get_inv_twiddles(trace_root, self.unextended_length());

        // move register traces into polys
        std::mem::swap(&mut self.registers, &mut self.polys);

        // extend all registers
        let domain_size = self.domain_size();
        for poly in self.polys.iter_mut() {
            // interpolate register trace into a polynomial
            fft::interpolate_poly(poly, &trace_twiddles, true);

            // allocate space to hold extended evaluations and copy the polynomial into it
            let mut register = vec![field::ZERO; domain_size];
            register[..poly.len()].copy_from_slice(&poly);

            // evaluate the polynomial over extended domain
            fft::evaluate_poly(&mut register, &lde_twiddles, true);
            self.registers.push(register);
        }
    }

    // TRACE COMMITMENT
    // --------------------------------------------------------------------------------------------
    /// Puts the trace table into a Merkle tree such that each row of the table becomes
    /// a distinct leaf in the tree; values all registers at a given step are hashed
    /// together to form a single leaf value.
    pub fn commit(&mut self, hash: HashFunction) -> [u8; 32] {
        assert!(self.is_extended(), "trace table hasn't been extended yet");
        assert!(
            !self.is_committed(),
            "trace commitment has already been built"
        );

        let mut trace_state = vec![field::ZERO; self.register_count()];
        let mut hashed_states = uninit_vector::<[u8; 32]>(self.domain_size());
        for i in 0..self.domain_size() {
            for j in 0..trace_state.len() {
                trace_state[j] = self.registers[j][i];
            }
            hash(as_bytes(&trace_state), &mut hashed_states[i]);
        }
        let trace_tree = MerkleTree::new(hashed_states, hash);
        let trace_root = *trace_tree.root();

        self.trace_tree = Some(trace_tree);

        trace_root
    }
}
