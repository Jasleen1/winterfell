use common::{
    errors::VerifierError,
    stark::{fri_utils, Commitments, DeepValues, FriLayer, PublicCoin, StarkProof},
    utils::{log2, uninit_vector},
    ComputationContext,
};
use core::convert::TryFrom;
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::{
    field::{AsBytes, FieldElement},
    quartic,
};

// TYPES AND INTERFACES
// ================================================================================================

pub struct VerifierChannel {
    context: ComputationContext,
    commitments: Commitments,
    trace_proof: BatchMerkleProof,
    trace_queries: Vec<Vec<FieldElement>>,
    constraint_proof: BatchMerkleProof,
    deep_values: DeepValues,
    fri_proofs: Vec<BatchMerkleProof>,
    fri_queries: Vec<Vec<[FieldElement; 4]>>,
    fri_remainder: Vec<FieldElement>,
}

// VERIFIER CHANNEL IMPLEMENTATION
// ================================================================================================

impl VerifierChannel {
    /// Creates and returns a new verifier channel initialized from the specified `proof`.
    pub fn new(proof: StarkProof) -> Self {
        // build context ------------------------------------------------------
        let context = proof.context;
        let trace_width = proof.deep_values.trace_at_z1.len();
        let trace_length =
            usize::pow(2, context.lde_domain_depth as u32) / context.options.blowup_factor();
        let context = ComputationContext::new(
            trace_width,
            trace_length,
            context.ce_blowup_factor as usize,
            context.options,
        );

        // build trace proof --------------------------------------------------
        let queries = proof.queries;
        let hash_fn = context.options().hash_fn();
        let trace_proof = build_trace_proof(
            &queries.trace_states,
            queries.trace_paths,
            context.lde_domain_size(),
            hash_fn,
        );

        // build FRI proofs ----------------------------------------------------
        let (fri_proofs, fri_queries) = build_fri_proofs(proof.fri_proof.layers, hash_fn);

        VerifierChannel {
            context,
            commitments: proof.commitments,
            deep_values: proof.deep_values,
            trace_proof,
            trace_queries: queries.trace_states,
            constraint_proof: queries.constraint_proof,
            fri_proofs,
            fri_queries,
            fri_remainder: proof.fri_proof.rem_values,
        }
    }

    /// Reads proof context from the channel.
    pub fn read_context(&self) -> &ComputationContext {
        &self.context
    }

    /// Returns trace polynomial evaluations at OOD points z and z * g, where g is the generator
    /// of the LDE domain.
    pub fn read_deep_values(&self) -> &DeepValues {
        &self.deep_values
    }

    /// Returns trace states at the specified positions. This also checks if the
    /// trace states are valid against the trace commitment sent by the prover.
    pub fn read_trace_states(
        &self,
        positions: &[usize],
    ) -> Result<&[Vec<FieldElement>], VerifierError> {
        // make sure the states included in the proof correspond to the trace commitment
        if !MerkleTree::verify_batch(
            &self.commitments.trace_root,
            positions,
            &self.trace_proof,
            self.context.options().hash_fn(),
        ) {
            return Err(VerifierError::TraceQueryDoesNotMatchCommitment);
        }

        Ok(&self.trace_queries)
    }

    /// Returns constraint evaluations at the specified positions. THis also checks if the
    /// constraint evaluations are valid against the constraint commitment sent by the prover.
    pub fn read_constraint_evaluations(
        &self,
        positions: &[usize],
    ) -> Result<Vec<FieldElement>, VerifierError> {
        let c_positions = map_trace_to_constraint_positions(positions);
        if !MerkleTree::verify_batch(
            &self.commitments.constraint_root,
            &c_positions,
            &self.constraint_proof,
            self.context.options().hash_fn(),
        ) {
            return Err(VerifierError::ConstraintQueryDoesNotMatchCommitment);
        }

        // build constraint evaluation values from the leaves of constraint Merkle proof
        let mut evaluations: Vec<FieldElement> = Vec::with_capacity(positions.len());
        let leaves = &self.constraint_proof.values;
        for &position in positions.iter() {
            let leaf_idx = c_positions.iter().position(|&v| v == position / 2).unwrap();
            let element_start = (position % 2) * 16;
            let element_bytes = &leaves[leaf_idx][element_start..(element_start + 16)];
            evaluations.push(FieldElement::try_from(element_bytes).unwrap());
        }

        Ok(evaluations)
    }

    /// Returns FRI query values at the specified positions from the FRI layer at the
    /// specified depth. This also checks if the values are valid against the FRI layer
    /// commitment sent by the prover.
    pub fn read_fri_queries(
        &self,
        depth: usize,
        positions: &[usize],
    ) -> Result<&[[FieldElement; 4]], String> {
        let layer_root = self.commitments.fri_roots[depth];
        let layer_proof = &self.fri_proofs[depth];
        if !MerkleTree::verify_batch(
            &layer_root,
            &positions,
            &layer_proof,
            self.context.options().hash_fn(),
        ) {
            return Err(format!(
                "FRI queries did not match the commitment at layer {}",
                depth
            ));
        }

        Ok(&self.fri_queries[depth])
    }

    /// Reads FRI remainder values (last FRI layer). This also checks that the remainder is
    /// valid against the commitment sent by the prover.
    pub fn read_fri_remainder(&self) -> Result<&[FieldElement], String> {
        // build remainder Merkle tree
        let hash_fn = self.context.options().hash_fn();
        let remainder_values = quartic::transpose(&self.fri_remainder, 1);
        let hashed_values = fri_utils::hash_values(&remainder_values, hash_fn);
        let remainder_tree = MerkleTree::new(hashed_values, hash_fn);

        // make sure the root of the tree matches the committed root of the last layer
        let committed_root = self.commitments.fri_roots.last().unwrap();
        if committed_root != remainder_tree.root() {
            return Err(String::from("FRI remainder did not match the commitment"));
        }

        Ok(&self.fri_remainder)
    }
}

// PUBLIC COIN IMPLEMENTATION
// ================================================================================================
impl PublicCoin for VerifierChannel {
    fn context(&self) -> &ComputationContext {
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
fn build_trace_proof(
    trace_states: &[Vec<FieldElement>],
    trace_paths: Vec<Vec<[u8; 32]>>,
    lde_domain_size: usize,
    hash_fn: HashFunction,
) -> BatchMerkleProof {
    let mut hashed_states = uninit_vector::<[u8; 32]>(trace_states.len());
    #[allow(clippy::needless_range_loop)]
    for i in 0..trace_states.len() {
        hash_fn(trace_states[i].as_slice().as_bytes(), &mut hashed_states[i]);
    }

    BatchMerkleProof {
        nodes: trace_paths,
        values: hashed_states,
        depth: log2(lde_domain_size) as u8,
    }
}

fn build_fri_proofs(
    layers: Vec<FriLayer>,
    hash_fn: HashFunction,
) -> (Vec<BatchMerkleProof>, Vec<Vec<[FieldElement; 4]>>) {
    let mut fri_queries = Vec::with_capacity(layers.len());
    let mut fri_proofs = Vec::with_capacity(layers.len());
    for layer in layers.into_iter() {
        fri_proofs.push(BatchMerkleProof {
            values: fri_utils::hash_values(&layer.values, hash_fn),
            nodes: layer.paths.clone(),
            depth: layer.depth,
        });
        fri_queries.push(layer.values);
    }

    (fri_proofs, fri_queries)
}

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
