use crate::{folding::quartic, FriOptions, FriProof, PublicCoin, VerifierError};
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::field::FieldElement;
use std::marker::PhantomData;

type Bytes = Vec<u8>;

// VERIFIER CHANNEL TRAIT
// ================================================================================================

pub trait VerifierChannel<E: FieldElement>: PublicCoin {
    /// Returns FRI query values at the specified positions from the FRI layer at the
    /// specified index. This also checks if the values are valid against the FRI layer
    /// commitment sent by the prover.
    fn read_layer_queries(
        &self,
        layer_idx: usize,
        positions: &[usize],
    ) -> Result<Vec<[E; 4]>, VerifierError> {
        let layer_root = self.fri_layer_commitments()[layer_idx];
        let layer_proof = &self.fri_layer_proofs()[layer_idx];
        if !MerkleTree::verify_batch(&layer_root, &positions, &layer_proof, self.hash_fn()) {
            return Err(VerifierError::LayerCommitmentMismatch(layer_idx));
        }

        // convert query bytes into field elements of appropriate type
        let mut queries = Vec::new();
        for query_bytes in self.fri_layer_queries()[layer_idx].iter() {
            let mut query = [E::ZERO; 4];
            E::read_into(query_bytes, &mut query)
                .map_err(|msg| VerifierError::LayerDeserializationError(layer_idx, msg))?;
            queries.push(query);
        }

        Ok(queries)
    }

    /// Reads FRI remainder values (last FRI layer). This also checks that the remainder is
    /// valid against the commitment sent by the prover.
    fn read_remainder(&self) -> Result<Vec<E>, VerifierError> {
        // convert remainder bytes into field elements of appropriate type
        let remainder = E::read_to_vec(&self.fri_remainder())
            .map_err(VerifierError::RemainderDeserializationError)?;

        // build remainder Merkle tree
        let remainder_values = quartic::transpose(&remainder, 1);
        let hashed_values = quartic::hash_values(&remainder_values, self.hash_fn());
        let remainder_tree = MerkleTree::new(hashed_values, self.hash_fn());

        // make sure the root of the tree matches the committed root of the last layer
        let committed_root = self.fri_layer_commitments().last().unwrap();
        if committed_root != remainder_tree.root() {
            return Err(VerifierError::RemainderCommitmentMismatch);
        }

        Ok(remainder)
    }

    /// Decomposes FRI proof struct into batch Merkle proofs and query values for each
    /// FRI layer, as well as remainder (the last FRI layer).
    fn parse_fri_proof(
        proof: FriProof,
        hash_fn: HashFunction,
    ) -> (Vec<BatchMerkleProof>, Vec<Vec<Vec<u8>>>, Vec<u8>) {
        let mut fri_queries = Vec::with_capacity(proof.layers.len());
        let mut fri_proofs = Vec::with_capacity(proof.layers.len());
        for layer in proof.layers.into_iter() {
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

        (fri_proofs, fri_queries, proof.rem_values)
    }

    fn num_fri_partitions(&self) -> usize {
        if self.fri_partitioned() {
            self.fri_remainder().len() / E::ELEMENT_BYTES
        } else {
            1
        }
    }

    fn fri_layer_proofs(&self) -> &[BatchMerkleProof];
    fn fri_layer_queries(&self) -> &[Vec<Bytes>];
    fn fri_remainder(&self) -> &[u8];
    fn fri_partitioned(&self) -> bool;
}

// DEFAULT VERIFIER CHANNEL IMPLEMENTATION
// ================================================================================================

pub struct DefaultVerifierChannel<E: FieldElement> {
    commitments: Vec<[u8; 32]>,
    proofs: Vec<BatchMerkleProof>,
    queries: Vec<Vec<Bytes>>,
    remainder: Bytes,
    partitioned: bool,
    hash_fn: HashFunction,
    _marker: PhantomData<E>,
}

impl<E: FieldElement> DefaultVerifierChannel<E> {
    /// Builds a new verifier channel from the specified parameters.
    pub fn new(proof: FriProof, commitments: Vec<[u8; 32]>, options: &FriOptions) -> Self {
        let hash_fn = options.hash_fn();
        let partitioned = proof.partitioned;
        let (proofs, queries, remainder) = Self::parse_fri_proof(proof, hash_fn);

        DefaultVerifierChannel {
            commitments,
            proofs,
            queries,
            remainder,
            partitioned,
            hash_fn,
            _marker: PhantomData,
        }
    }
}

impl<E: FieldElement> VerifierChannel<E> for DefaultVerifierChannel<E> {
    fn fri_layer_proofs(&self) -> &[BatchMerkleProof] {
        &self.proofs
    }

    fn fri_layer_queries(&self) -> &[Vec<Bytes>] {
        &self.queries
    }

    fn fri_remainder(&self) -> &[u8] {
        &self.remainder
    }

    fn fri_partitioned(&self) -> bool {
        self.partitioned
    }
}

impl<E: FieldElement> PublicCoin for DefaultVerifierChannel<E> {
    fn fri_layer_commitments(&self) -> &[[u8; 32]] {
        &self.commitments
    }

    fn hash_fn(&self) -> HashFunction {
        self.hash_fn
    }
}
