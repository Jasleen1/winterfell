use common::{
    errors::VerifierError,
    proof::{Commitments, OodEvaluationFrame, StarkProof},
    ComputationContext, EvaluationFrame, ProofOptions, PublicCoin,
};
use core::convert::TryFrom;
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use fri::{self, VerifierChannel as FriVerifierChannel};
use math::{
    field::{BaseElement, FieldElement},
    utils::log2,
};
use std::convert::TryInto;
use std::marker::PhantomData;

// TYPES AND INTERFACES
// ================================================================================================

type Bytes = Vec<u8>;

pub struct VerifierChannel<E: FieldElement + From<BaseElement>> {
    context: ComputationContext,
    commitments: Commitments,
    trace_proof: BatchMerkleProof,
    trace_queries: Vec<Bytes>,
    constraint_proof: BatchMerkleProof,
    ood_frame: OodEvaluationFrame,
    fri_layer_proofs: Vec<BatchMerkleProof>,
    fri_layer_queries: Vec<Vec<Bytes>>,
    fri_remainder: Bytes,
    fri_partitioned: bool,
    query_seed: [u8; 32],
    _marker: PhantomData<E>,
}

// VERIFIER CHANNEL IMPLEMENTATION
// ================================================================================================

impl<E: FieldElement + From<BaseElement>> VerifierChannel<E> {
    /// Creates and returns a new verifier channel initialized from the specified `proof`.
    pub fn new(context: ComputationContext, proof: StarkProof) -> Result<Self, VerifierError> {
        // build trace proof --------------------------------------------------
        let queries = proof.queries;
        let hash_fn = context.options().hash_fn();
        let trace_proof = build_trace_proof(
            &queries.trace_states,
            queries.trace_paths,
            context.lde_domain_size(),
            hash_fn,
        );

        // parse FRI proofs ---------------------------------------------------
        let fri_partitioned = proof.fri_proof.partitioned;
        let (fri_layer_proofs, fri_layer_queries, fri_remainder) =
            Self::parse_fri_proof(proof.fri_proof, hash_fn);

        // build query seed ---------------------------------------------------
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
            fri_layer_proofs,
            fri_layer_queries,
            fri_remainder,
            fri_partitioned,
            query_seed,
            _marker: PhantomData,
        })
    }

    /// Returns trace polynomial evaluations at OOD points z and z * g, where g is the generator
    /// of the LDE domain.
    pub fn read_ood_frame(&self) -> Result<EvaluationFrame<E>, VerifierError> {
        let current = E::read_into_vec(&self.ood_frame.trace_at_z1)
            .map_err(|_| VerifierError::OodFrameDeserializationFailed)?;
        let next = E::read_into_vec(&self.ood_frame.trace_at_z2)
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
}

impl<E: FieldElement + From<BaseElement>> fri::VerifierChannel<E> for VerifierChannel<E> {
    fn fri_layer_proofs(&self) -> &[BatchMerkleProof] {
        &self.fri_layer_proofs
    }

    fn fri_layer_queries(&self) -> &[Vec<Bytes>] {
        &self.fri_layer_queries
    }

    fn fri_remainder(&self) -> &[u8] {
        &self.fri_remainder
    }

    fn fri_partitioned(&self) -> bool {
        self.fri_partitioned
    }
}

// PUBLIC COIN IMPLEMENTATION
// ================================================================================================
impl<E: FieldElement + From<BaseElement>> PublicCoin for VerifierChannel<E> {
    fn context(&self) -> &ComputationContext {
        &self.context
    }

    fn constraint_seed(&self) -> [u8; 32] {
        self.commitments.trace_root
    }

    fn composition_seed(&self) -> [u8; 32] {
        self.commitments.constraint_root
    }

    fn query_seed(&self) -> [u8; 32] {
        self.query_seed
    }
}

impl<E: FieldElement + From<BaseElement>> fri::PublicCoin for VerifierChannel<E> {
    fn fri_layer_commitments(&self) -> &[[u8; 32]] {
        &self.commitments.fri_roots
    }

    fn hash_fn(&self) -> HashFunction {
        self.context.options().hash_fn()
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
    let mut hashed_states = vec![[0u8; 32]; trace_states.len()];
    for (trace_state, state_hash) in trace_states.iter().zip(hashed_states.iter_mut()) {
        hash_fn(trace_state, state_hash);
    }

    BatchMerkleProof {
        nodes: trace_paths,
        values: hashed_states,
        depth: log2(lde_domain_size) as u8,
    }
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
