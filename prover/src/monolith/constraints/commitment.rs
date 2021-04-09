use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::{field::FieldElement, utils::transmute_vector};
use std::marker::PhantomData;

// CONSTRAINT COMMITMENT
// ================================================================================================

pub struct ConstraintCommitment<E: FieldElement> {
    tree: MerkleTree,
    _marker1: PhantomData<E>,
}

impl<E: FieldElement> ConstraintCommitment<E> {
    /// Commits to the constraint evaluations by putting them into a Merkle tree; since
    /// evaluations for a specific step are compressed into a single field element, we try
    /// to put multiple evaluations into a single leaf whenever possible.
    pub fn new(evaluations: Vec<E>, hash_fn: HashFunction) -> ConstraintCommitment<E> {
        assert!(
            evaluations.len().is_power_of_two(),
            "number of values must be a power of 2"
        );

        // call evaluations_per_leaf() to make sure that the whole number of evaluations fits
        // into a single leaf
        let _ = Self::evaluations_per_leaf();

        // reinterpret vector of field elements as a vector of 32-byte arrays;
        let evaluations = transmute_vector::<u8, 32>(E::elements_into_bytes(evaluations));
        // build Merkle tree out of evaluations
        ConstraintCommitment {
            tree: MerkleTree::new(evaluations, hash_fn),
            _marker1: PhantomData,
        }
    }

    /// Returns the root of the commitment Merkle tree.
    pub fn root(&self) -> [u8; 32] {
        *self.tree.root()
    }

    /// Returns the depth of the commitment Merkle tree.
    pub fn tree_depth(&self) -> usize {
        self.tree.depth()
    }

    /// Returns constraint evaluations at the specified positions along with Merkle
    /// authentication paths from the root of the commitment to these evaluations.
    /// Since evaluations are compressed into a single field element, the are already
    /// included in Merkle authentication paths.
    pub fn query(self, trace_positions: &[usize]) -> BatchMerkleProof {
        // first, map trace positions to the corresponding positions in the constraint tree;
        // we do this because multiple constraint evaluations may be stored in a single leaf
        let evaluations_per_leaf = Self::evaluations_per_leaf();
        let mut constraint_positions = Vec::with_capacity(trace_positions.len());
        for &position in trace_positions.iter() {
            let cp = position / evaluations_per_leaf;
            if !constraint_positions.contains(&cp) {
                constraint_positions.push(cp);
            }
        }

        // build Merkle authentication paths to the leaves specified by constraint positions
        self.tree.prove_batch(&constraint_positions)
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    /// Computes number of evaluations which can fit into a single Merkle tree leaf. Leaves
    /// are assumed to be 32-bytes in size.
    fn evaluations_per_leaf() -> usize {
        assert!(
            E::ELEMENT_BYTES.is_power_of_two(),
            "elements with number of bytes which are not powers of 2 are not supported yet"
        );
        let result = 32 / E::ELEMENT_BYTES;
        assert!(
            result > 0,
            "field elements larger than 32 bytes are not supported"
        );
        result
    }
}
