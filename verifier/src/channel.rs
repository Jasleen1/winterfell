use common::{
    stark::{Commitments, DeepValues, FriProof, ProofContext, PublicCoin, Queries, StarkProof},
    utils::{as_bytes, log2, uninit_vector},
};
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::field;

// TYPES AND INTERFACES
// ================================================================================================

pub struct VerifierChannel {
    context: ProofContext,
    commitments: Commitments,
    queries: Queries,
    deep_values: DeepValues,
    fri_proof: FriProof,
}

// VERIFIER CHANNEL IMPLEMENTATION
// ================================================================================================

impl VerifierChannel {
    pub fn new(proof: StarkProof) -> Self {
        let context = proof.context;
        let trace_width = proof.deep_values.trace_at_z1.len();
        let trace_length =
            usize::pow(2, context.lde_domain_depth as u32) / context.options.blowup_factor();

        VerifierChannel {
            context: ProofContext::new(
                trace_width,
                trace_length,
                context.max_constraint_degree as usize,
                context.options.clone(),
            ),
            commitments: proof.commitments,
            deep_values: proof.deep_values,
            queries: proof.queries,
            fri_proof: proof.fri_proof,
        }
    }

    pub fn read_context(&self) -> &ProofContext {
        &self.context
    }

    pub fn read_deep_values(&self) -> &DeepValues {
        &self.deep_values
    }

    pub fn read_trace_states(&self, positions: &[usize]) -> Result<&[Vec<u128>], String> {
        let queries = &self.queries;
        let hash_fn = self.context.options().hash_fn();

        let mut hashed_states = uninit_vector::<[u8; 32]>(queries.trace_states.len());
        #[allow(clippy::needless_range_loop)]
        for i in 0..queries.trace_states.len() {
            hash_fn(as_bytes(&queries.trace_states[i]), &mut hashed_states[i]);
        }

        let trace_proof = BatchMerkleProof {
            nodes: queries.trace_paths.clone(),
            values: hashed_states,
            depth: log2(self.context.lde_domain_size()) as u8,
        };

        if !MerkleTree::verify_batch(
            &self.commitments.trace_root,
            positions,
            &trace_proof,
            hash_fn,
        ) {
            return Err(String::from("verification of trace Merkle proof failed"));
        }

        Ok(&queries.trace_states)
    }

    pub fn read_constraint_evaluations(&self, positions: &[usize]) -> Result<Vec<u128>, String> {
        let queries = &self.queries;
        let hash_fn = self.context.options().hash_fn();

        let c_positions = map_trace_to_constraint_positions(positions);
        if !MerkleTree::verify_batch(
            &self.commitments.constraint_root,
            &c_positions,
            &queries.constraint_proof,
            hash_fn,
        ) {
            return Err(String::from(
                "verification of constraint Merkle proof failed",
            ));
        }

        // build constraint evaluation values from the leaves of constraint Merkle proof
        let mut evaluations: Vec<u128> = Vec::with_capacity(positions.len());
        let leaves = &queries.constraint_proof.values;
        for &position in positions.iter() {
            let leaf_idx = c_positions.iter().position(|&v| v == position / 2).unwrap();
            let element_start = (position % 2) * 16;
            let element_bytes = &leaves[leaf_idx][element_start..(element_start + 16)];
            evaluations.push(field::from_bytes(element_bytes));
        }

        Ok(evaluations)
    }

    pub fn read_fri_queries(
        &self,
        depth: usize,
        positions: &[usize],
    ) -> Result<&[[u128; 4]], String> {
        let layer = &self.fri_proof.layers[depth];
        let hash_fn = self.context.options().hash_fn();

        let mut hashed_values: Vec<[u8; 32]> = uninit_vector(layer.values.len());
        for i in 0..layer.values.len() {
            hash_fn(as_bytes(&layer.values[i]), &mut hashed_values[i]);
        }

        let proof = BatchMerkleProof {
            values: hashed_values,
            nodes: layer.nodes.clone(),
            depth: layer.depth,
        };
        if !MerkleTree::verify_batch(&layer.root, &positions, &proof, hash_fn) {
            return Err(format!(
                "verification of Merkle proof failed at layer {}",
                depth
            ));
        }

        Ok(&self.fri_proof.layers[depth].values)
    }

    pub fn read_fri_remainder(&self) -> &[u128] {
        &self.fri_proof.rem_values
    }
}

// PUBLIC COIN IMPLEMENTATION
// ================================================================================================
impl PublicCoin for VerifierChannel {
    fn context(&self) -> &ProofContext {
        &self.context
    }

    fn constraint_seed(&self) -> [u8; 32] {
        self.commitments.trace_root
    }

    fn composition_seed(&self) -> [u8; 32] {
        self.commitments.constraint_root
    }

    fn fri_layer_seed(&self, layer_depth: usize) -> [u8; 32] {
        self.commitments.fri_roots[layer_depth]
    }

    fn query_seed(&self) -> [u8; 32] {
        build_query_seed(
            &self.commitments.fri_roots,
            self.context.options().hash_fn(),
        )
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn map_trace_to_constraint_positions(positions: &[usize]) -> Vec<usize> {
    let mut result = Vec::with_capacity(positions.len());
    for &position in positions.iter() {
        let cp = position / 2;
        if !result.contains(&cp) {
            result.push(cp);
        }
    }
    result
}

fn build_query_seed(fri_roots: &[[u8; 32]], hash_fn: HashFunction) -> [u8; 32] {
    // combine roots of all FIR layers into a single array of bytes
    let mut root_bytes: Vec<u8> = Vec::with_capacity(fri_roots.len() * 32);
    for root in fri_roots.iter() {
        root.iter().for_each(|&v| root_bytes.push(v));
    }

    // hash the array of bytes into a single 32-byte value
    let mut query_seed = [0u8; 32];
    hash_fn(&root_bytes, &mut query_seed);

    /*
    // TODO: verify proof of work
    let seed = match utils::verify_pow_nonce(seed, proof.pow_nonce(), &self.options) {
        Ok(seed) => seed,
        Err(msg) => return Err(msg)
    };
    */

    query_seed
}
