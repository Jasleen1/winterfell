use crate::ProofOptions;
use crypto::BatchMerkleProof;
use serde::{Deserialize, Serialize};

// CONSTANTS
// ================================================================================================

const GRINDING_CONTRIBUTION_FLOOR: u32 = 80;

// TYPES AND INTERFACES
// ================================================================================================

// TODO: custom serialization should reduce size by 5% - 10%
#[derive(Clone, Serialize, Deserialize)]
pub struct StarkProof {
    pub context: Context,
    pub commitments: Commitments,
    pub queries: Queries,
    pub ood_frame: OodEvaluationFrame,
    pub fri_proof: FriProof,
    pub pow_nonce: u64,
}

// TODO: this should be replaced by ProofContext
#[derive(Clone, Serialize, Deserialize)]
pub struct Context {
    pub trace_width: u8,
    pub lde_domain_depth: u8,
    pub ce_blowup_factor: u8,
    pub field_modulus_bytes: Vec<u8>,
    pub field_extension_factor: u32,
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
    pub trace_states: Vec<Vec<u8>>,
    pub constraint_proof: BatchMerkleProof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriLayer {
    pub values: Vec<Vec<u8>>,
    pub paths: Vec<Vec<[u8; 32]>>,
    pub depth: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriProof {
    pub layers: Vec<FriLayer>,
    pub rem_values: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OodEvaluationFrame {
    pub trace_at_z1: Vec<u8>,
    pub trace_at_z2: Vec<u8>,
}

// STARK PROOF IMPLEMENTATION
// ================================================================================================
impl StarkProof {
    pub fn security_level(&self, optimistic: bool) -> u32 {
        let options = &self.context.options;

        // conjectured security requires half the queries as compared to proven security
        let num_queries = if optimistic {
            options.num_queries()
        } else {
            options.num_queries() / 2
        };

        let one_over_rho =
            (options.blowup_factor() / self.context.ce_blowup_factor as usize) as u32;
        let security_per_query = 31 - one_over_rho.leading_zeros(); // same as log2(one_over_rho)
        let mut result = security_per_query * num_queries as u32;

        // include grinding factor contributions only for proofs adequate security
        if result >= GRINDING_CONTRIBUTION_FLOOR {
            result += options.grinding_factor();
        }

        // Provided by the collision resistance (CR) of the hash function we use
        // TODO: make this dynamic based on the hash function used
        let cr_security = 128;

        // Parse field modulus from field_modulus_bytes
        let mut output_array = [0u8; std::mem::size_of::<u128>()];
        output_array[std::mem::size_of::<u128>() - self.context.field_modulus_bytes.len()..]
            .copy_from_slice(&self.context.field_modulus_bytes);
        let field_modulus = u128::from_be_bytes(output_array);

        // Compute floor(log_2(field size))
        let field_modulus_bits = ((self.context.field_modulus_bytes.len() * 8) as u32)
            - field_modulus.leading_zeros()
            - 1;

        // field_modulus_bits * field_extension_factor - log2(extended trace length)
        let max_fri_security = field_modulus_bits * self.context.field_extension_factor
            - self.context.lde_domain_depth as u32;

        std::cmp::min(std::cmp::min(result, max_fri_security), cr_security)
    }
}
