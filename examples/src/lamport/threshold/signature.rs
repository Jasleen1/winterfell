use crate::{
    lamport::signature::PublicKey,
    utils::{bytes_to_node, TreeNode},
};
use prover::crypto::{hash::rescue_d, MerkleTree};

// AGGREGATED PUBLIC KEY
// ================================================================================================

pub struct AggPublicKey {
    keys: Vec<PublicKey>,
    tree: MerkleTree,
}

impl AggPublicKey {
    pub fn new(mut keys: Vec<PublicKey>) -> Self {
        // sort keys in ascending order
        keys.sort();

        // convert keys to arrays of bytes; each key is hashed using Rescue hash function; the
        // initial hashing makes the AIR design a little simpler
        let mut leaves: Vec<[u8; 32]> = Vec::new();
        for key in keys.iter() {
            let mut result = [0u8; 32];
            rescue_d(&key.to_bytes(), &mut result);
            leaves.push(result);
        }

        // pad the list of keys with zero keys to make sure the number of leaves is greater than
        // the number of keys and is a power of two
        let num_leaves = if leaves.len().is_power_of_two() {
            (leaves.len() + 1).next_power_of_two()
        } else {
            leaves.len().next_power_of_two()
        };
        for _ in leaves.len()..num_leaves {
            let mut result = [0u8; 32];
            rescue_d(&[0; 32], &mut result);
            leaves.push(result);
        }

        // build a Merkle tree of all leaves
        let tree = MerkleTree::new(leaves, rescue_d);

        AggPublicKey { keys, tree }
    }

    /// Returns a 32-byte representation of the aggregated public key.
    pub fn root(&self) -> [u8; 32] {
        *self.tree.root()
    }

    /// Returns the number of individual keys aggregated into this key.
    pub fn num_keys(&self) -> usize {
        self.keys.len()
    }

    /// Returns number of leaves in the aggregated public key; this will always be greater
    // than the number of individual keys.
    pub fn num_leaves(&self) -> usize {
        self.tree.leaves().len()
    }

    /// Returns an individual key at the specified index, if one exists.
    pub fn get_key(&self, index: usize) -> Option<PublicKey> {
        if index < self.keys.len() {
            Some(self.keys[index])
        } else {
            None
        }
    }

    /// Returns a Merkle path to the specified leaf.
    pub fn get_leaf_path(&self, index: usize) -> Vec<TreeNode> {
        let path = self.tree.prove(index);
        path.iter().map(|bytes| bytes_to_node(*bytes)).collect()
    }
}
