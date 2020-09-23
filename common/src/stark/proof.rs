use super::ProofOptions;
use crypto::BatchMerkleProof;
use serde::{Deserialize, Serialize};

// TYPES AND INTERFACES
// ================================================================================================

// TODO: custom serialization should reduce size by 5% - 10%
#[derive(Clone, Serialize, Deserialize)]
pub struct StarkProof {
    pub lde_domain_depth: u8,
    pub trace_root: [u8; 32],
    pub trace_nodes: Vec<Vec<[u8; 32]>>,
    pub trace_states: Vec<Vec<u128>>,
    pub constraint_root: [u8; 32],
    pub constraint_proof: BatchMerkleProof,
    pub max_constraint_degree: u8,
    pub deep_values: DeepValues,
    pub fri_proof: FriProof,
    pub pow_nonce: u64,
    pub options: ProofOptions,
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
    // SECURITY LEVEL
    // -------------------------------------------------------------------------------------------
    pub fn security_level(&self, optimistic: bool) -> u32 {
        // conjectured security requires half the queries as compared to proven security
        let num_queries = if optimistic {
            self.options.num_queries()
        } else {
            self.options.num_queries() / 2
        };

        let one_over_rho = (self.options.blowup_factor()
            / self.max_constraint_degree.next_power_of_two() as usize)
            as u32;
        let security_per_query = 31 - one_over_rho.leading_zeros(); // same as log2(one_over_rho)

        let mut result1 = security_per_query * num_queries as u32;
        if result1 >= 80 {
            result1 += self.options.grinding_factor() as u32;
        }

        // log2(field size) - log2(extended trace length)
        let result2 = (128 - self.lde_domain_depth) as u32;

        std::cmp::min(result1, result2)
    }
}
