use super::{ProofOptions, TraceInfo};
use crate::utils::{as_bytes, uninit_vector};
use crypto::BatchMerkleProof;
use serde::{Deserialize, Serialize};

// TYPES AND INTERFACES
// ================================================================================================

// TODO: custom serialization should reduce size by 5% - 10%
#[derive(Clone, Serialize, Deserialize)]
pub struct StarkProof {
    domain_depth: u8,
    trace_root: [u8; 32],
    trace_nodes: Vec<Vec<[u8; 32]>>,
    trace_states: Vec<Vec<u128>>,
    constraint_root: [u8; 32],
    constraint_proof: BatchMerkleProof,
    deep_values: DeepValues,
    fri_proof: FriProof,
    pow_nonce: u64,
    options: ProofOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriLayer {
    pub root: [u8; 32],
    pub values: Vec<[u128; 4]>,
    pub nodes: Vec<Vec<[u8; 32]>>,
    pub depth: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriProof {
    pub layers: Vec<FriLayer>,
    pub rem_root: [u8; 32],
    pub rem_values: Vec<u128>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeepValues {
    pub trace_at_z1: Vec<u128>,
    pub trace_at_z2: Vec<u128>,
}

// STARK PROOF IMPLEMENTATION
// ================================================================================================
impl StarkProof {
    // CONSTRUCTOR
    // -------------------------------------------------------------------------------------------
    // TODO: would be good to re-factor proof structure so information could be written into it
    // gradually. Also, maybe it makes sense to move drawing of randomness logic here.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        trace_root: [u8; 32],
        trace_proof: BatchMerkleProof,
        trace_states: Vec<Vec<u128>>,
        constraint_root: [u8; 32],
        constraint_proof: BatchMerkleProof,
        deep_values: DeepValues,
        fri_proof: FriProof,
        pow_nonce: u64,
        options: ProofOptions,
    ) -> StarkProof {
        StarkProof {
            trace_root,
            domain_depth: trace_proof.depth,
            trace_nodes: trace_proof.nodes,
            trace_states,
            constraint_root,
            constraint_proof,
            deep_values,
            fri_proof,
            pow_nonce,
            options,
        }
    }

    // TRACE
    // -------------------------------------------------------------------------------------------

    pub fn trace_root(&self) -> &[u8; 32] {
        &self.trace_root
    }

    pub fn trace_proof(&self) -> BatchMerkleProof {
        let hash = self.options.hash_fn();
        let mut hashed_states = uninit_vector::<[u8; 32]>(self.trace_states.len());
        #[allow(clippy::needless_range_loop)]
        for i in 0..self.trace_states.len() {
            hash(as_bytes(&self.trace_states[i]), &mut hashed_states[i]);
        }

        BatchMerkleProof {
            nodes: self.trace_nodes.clone(),
            values: hashed_states,
            depth: self.domain_depth,
        }
    }

    pub fn trace_states(&self) -> &[Vec<u128>] {
        &self.trace_states
    }

    pub fn trace_info(&self) -> TraceInfo {
        let lde_domain_size = usize::pow(2, self.domain_depth as u32);
        let blowup = self.options.blowup_factor();
        let length = lde_domain_size / blowup;
        let width = self.trace_states[0].len();

        TraceInfo::new(width, length, blowup)
    }

    // CONSTRAINTS
    // -------------------------------------------------------------------------------------------

    pub fn constraint_root(&self) -> &[u8; 32] {
        &self.constraint_root
    }

    pub fn constraint_proof(&self) -> BatchMerkleProof {
        self.constraint_proof.clone()
    }

    // DEEP VALUES
    // -------------------------------------------------------------------------------------------
    pub fn get_state_at_z1(&self) -> &[u128] {
        &self.deep_values.trace_at_z1
    }

    pub fn get_state_at_z2(&self) -> &[u128] {
        &self.deep_values.trace_at_z2
    }

    // OTHER PROPERTIES
    // -------------------------------------------------------------------------------------------

    pub fn fri_proof(&self) -> &FriProof {
        &self.fri_proof
    }

    pub fn pow_nonce(&self) -> u64 {
        self.pow_nonce
    }

    pub fn options(&self) -> &ProofOptions {
        &self.options
    }
}
