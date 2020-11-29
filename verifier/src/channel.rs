use common::{
    errors::VerifierError,
    fri_utils,
    proof::{Commitments, FriLayer, OodEvaluationFrame, StarkProof},
    utils::{log2, uninit_vector},
    ComputationContext, EvaluationFrame, ProofOptions, PublicCoin,
};
use core::convert::TryFrom;
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::{
    field::{BaseElement, FieldElement},
    quartic,
};
use std::convert::TryInto;

// TYPES AND INTERFACES
// ================================================================================================

type Bytes = Vec<u8>;

pub struct VerifierChannel {
    context: ComputationContext,
    commitments: Commitments,
    trace_proof: BatchMerkleProof,
    trace_queries: Vec<Bytes>,
    constraint_proof: BatchMerkleProof,
    ood_frame: OodEvaluationFrame,
    fri_proofs: Vec<BatchMerkleProof>,
    fri_queries: Vec<Vec<Bytes>>,
    fri_remainder: Bytes,
    query_seed: [u8; 32],
}

// VERIFIER CHANNEL IMPLEMENTATION
// ================================================================================================

impl VerifierChannel {
    /// Creates and returns a new verifier channel initialized from the specified `proof`.
    pub fn new(proof: StarkProof) -> Result<Self, VerifierError> {
        // build context ------------------------------------------------------
        let context = proof.context;
        let trace_width = context.trace_width as usize;
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

        // build query seed ----------------------------------------------------
        let query_seed = build_query_seed(
            &proof.commitments.fri_roots,
            proof.pow_nonce,
            &context.options(),
        )?;

        Ok(VerifierChannel {
            context,
            commitments: proof.commitments,
            ood_frame: proof.ood_frame,
            trace_proof,
            trace_queries: queries.trace_states,
            constraint_proof: queries.constraint_proof,
            fri_proofs,
            fri_queries,
            fri_remainder: proof.fri_proof.rem_values,
            query_seed,
        })
    }

    /// Reads proof context from the channel.
    pub fn read_context(&self) -> &ComputationContext {
        &self.context
    }

    /// Returns trace polynomial evaluations at OOD points z and z * g, where g is the generator
    /// of the LDE domain.
    pub fn read_ood_frame<E: FieldElement>(&self) -> Result<EvaluationFrame<E>, VerifierError> {
        let current = E::read_to_vec(&self.ood_frame.trace_at_z1)
            .map_err(|_| VerifierError::OodFrameDeserializationFailed)?;

        let next = E::read_to_vec(&self.ood_frame.trace_at_z2)
            .map_err(|_| VerifierError::OodFrameDeserializationFailed)?;

        Ok(EvaluationFrame { current, next })
    }

    /// Returns trace states at the specified positions. This also checks if the
    /// trace states are valid against the trace commitment sent by the prover.
    pub fn read_trace_states(
        &self,
        positions: &[usize],
    ) -> Result<Vec<Vec<BaseElement>>, VerifierError> {
        // make sure the states included in the proof correspond to the trace commitment
        if !MerkleTree::verify_batch(
            &self.commitments.trace_root,
            positions,
            &self.trace_proof,
            self.context.options().hash_fn(),
        ) {
            return Err(VerifierError::TraceQueryDoesNotMatchCommitment);
        }

        // convert query bytes into field elements of appropriate type
        let mut states = Vec::new();
        for state_bytes in self.trace_queries.iter() {
            let mut trace_state = vec![BaseElement::ZERO; self.context.trace_width()];
            BaseElement::read_into(state_bytes, &mut trace_state)
                .map(|_| states.push(trace_state))
                .map_err(|_| VerifierError::TraceQueryDeserializationFailed)?
        }

        Ok(states)
    }

    /// Returns constraint evaluations at the specified positions. THis also checks if the
    /// constraint evaluations are valid against the constraint commitment sent by the prover.
    pub fn read_constraint_evaluations(
        &self,
        positions: &[usize],
    ) -> Result<Vec<BaseElement>, VerifierError> {
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
        let mut evaluations: Vec<BaseElement> = Vec::with_capacity(positions.len());
        let leaves = &self.constraint_proof.values;
        for &position in positions.iter() {
            let leaf_idx = c_positions.iter().position(|&v| v == position / 2).unwrap();
            let element_start = (position % 2) * 16;
            let element_bytes = &leaves[leaf_idx][element_start..(element_start + 16)];
            evaluations.push(BaseElement::try_from(element_bytes).unwrap());
        }

        Ok(evaluations)
    }

    /// Returns FRI query values at the specified positions from the FRI layer at the
    /// specified depth. This also checks if the values are valid against the FRI layer
    /// commitment sent by the prover.
    pub fn read_fri_queries<E: FieldElement>(
        &self,
        depth: usize,
        positions: &[usize],
    ) -> Result<Vec<[E; 4]>, String> {
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

        // convert query bytes into field elements of appropriate type
        let mut queries = Vec::new();
        for query_bytes in self.fri_queries[depth].iter() {
            let mut query = [E::ZERO; 4];
            E::read_into(query_bytes, &mut query)?;
            queries.push(query);
        }

        Ok(queries)
    }

    /// Reads FRI remainder values (last FRI layer). This also checks that the remainder is
    /// valid against the commitment sent by the prover.
    pub fn read_fri_remainder<E: FieldElement>(&self) -> Result<Vec<E>, String> {
        // convert remainder bytes into field elements of appropriate type
        let remainder = E::read_to_vec(&self.fri_remainder)?;

        // build remainder Merkle tree
        let hash_fn = self.context.options().hash_fn();
        let remainder_values = quartic::transpose(&remainder, 1);
        let hashed_values = fri_utils::hash_values(&remainder_values, hash_fn);
        let remainder_tree = MerkleTree::new(hashed_values, hash_fn);

        // make sure the root of the tree matches the committed root of the last layer
        let committed_root = self.commitments.fri_roots.last().unwrap();
        if committed_root != remainder_tree.root() {
            return Err(String::from("FRI remainder did not match the commitment"));
        }

        Ok(remainder)
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
        self.query_seed
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_trace_proof(
    trace_states: &[Bytes],
    trace_paths: Vec<Vec<[u8; 32]>>,
    lde_domain_size: usize,
    hash_fn: HashFunction,
) -> BatchMerkleProof {
    let mut hashed_states = uninit_vector::<[u8; 32]>(trace_states.len());
    #[allow(clippy::needless_range_loop)]
    for i in 0..trace_states.len() {
        hash_fn(&trace_states[i], &mut hashed_states[i]);
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
) -> (Vec<BatchMerkleProof>, Vec<Vec<Vec<u8>>>) {
    let mut fri_queries = Vec::with_capacity(layers.len());
    let mut fri_proofs = Vec::with_capacity(layers.len());
    for layer in layers.into_iter() {
        let mut hashed_values = Vec::new();
        for value_bytes in layer.values.iter() {
            let mut buf = [0u8; 32];
            hash_fn(value_bytes, &mut buf);
            hashed_values.push(buf);
        }

        fri_proofs.push(BatchMerkleProof {
            values: hashed_values,
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

fn build_query_seed(
    fri_roots: &[[u8; 32]],
    nonce: u64,
    options: &ProofOptions,
) -> Result<[u8; 32], VerifierError> {
    let hash = options.hash_fn();

    // combine roots of all FIR layers into a single array of bytes
    let mut root_bytes: Vec<u8> = Vec::with_capacity(fri_roots.len() * 32);
    for root in fri_roots.iter() {
        root.iter().for_each(|&v| root_bytes.push(v));
    }

    // hash the array of bytes into a single 32-byte value
    let mut query_seed = [0u8; 32];
    hash(&root_bytes, &mut query_seed);

    // verify proof of work
    let mut input_bytes = [0; 64];
    input_bytes[0..32].copy_from_slice(&query_seed);
    input_bytes[56..].copy_from_slice(&nonce.to_le_bytes());

    hash(&input_bytes, &mut query_seed);

    let seed_head = u64::from_le_bytes(query_seed[..8].try_into().unwrap());
    if seed_head.trailing_zeros() < options.grinding_factor() {
        return Err(VerifierError::QuerySeedProofOfWorkVerificationFailed);
    }

    Ok(query_seed)
}
