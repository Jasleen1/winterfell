use super::ProofOptions;
use crypto::BatchMerkleProof;
use serde::{Deserialize, Serialize};

// TYPES AND INTERFACES
// ================================================================================================

// TODO: custom serialization should reduce size by 5% - 10%
#[derive(Clone, Serialize, Deserialize)]
pub struct StarkProof {
    pub context: Context,
    pub commitments: Commitments,
    pub queries: Queries,
    pub deep_values: DeepValues,
    pub fri_proof: FriProof,
    pub pow_nonce: u64,
}

// TODO: this should be replaced by ProofContext
#[derive(Clone, Serialize, Deserialize)]
pub struct Context {
    pub lde_domain_depth: u8,
    pub max_constraint_degree: u8,
    pub options: ProofOptions,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Commitments {
    pub trace_root: [u8; 32],
    pub constraint_root: [u8; 32],
    pub fri_roots: Vec<[u8; 32]>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Queries {
    pub trace_paths: Vec<Vec<[u8; 32]>>,
    pub trace_states: Vec<Vec<u128>>,
    pub constraint_proof: BatchMerkleProof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriLayer {
    pub root: [u8; 32], // TODO: remove roots as they are now in commitments
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
        let options = &self.context.options;

        // conjectured security requires half the queries as compared to proven security
        let num_queries = if optimistic {
            options.num_queries()
        } else {
            options.num_queries() / 2
        };

        let one_over_rho = (options.blowup_factor()
            / self.context.max_constraint_degree.next_power_of_two() as usize)
            as u32;
        let security_per_query = 31 - one_over_rho.leading_zeros(); // same as log2(one_over_rho)

        let mut result1 = security_per_query * num_queries as u32;
        if result1 >= 80 {
            result1 += options.grinding_factor() as u32;
        }

        // log2(field size) - log2(extended trace length)
        let result2 = (128 - self.context.lde_domain_depth) as u32;

        std::cmp::min(result1, result2)
    }
}
