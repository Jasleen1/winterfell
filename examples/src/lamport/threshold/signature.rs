use prover::crypto::{MerkleTree, hash::rescue_d};

use crate::{lamport::signature::PublicKey, utils::{TreeNode, bytes_to_node}};

pub struct AggPublicKey {
    keys: Vec<PublicKey>,
    tree: MerkleTree
}

impl AggPublicKey {

    pub fn new(mut keys: Vec<PublicKey>) -> Self {
        // sort keys in ascending order
        keys.sort_by(compare_pub_keys);

        // convert keys to arrays of bytes
        let mut leaves: Vec<[u8; 32]> = Vec::new();
        for key in keys.iter() {
            let mut result = [0u8; 32];
            rescue_d(&key.to_bytes(), &mut result);
            leaves.push(result);
        }

        // pad the list of keys to make sure number of leaves is a power of 2
        if !leaves.len().is_power_of_two() {
            let new_len = leaves.len().next_power_of_two();
            for _ in leaves.len()..new_len {
                let mut result = [0u8; 32];
                rescue_d(&[0; 32], &mut result);
                leaves.push(result);
            }
        }
        // build a Merkle tree of all leaves
        let tree = MerkleTree::new(leaves, rescue_d);

        AggPublicKey { keys, tree }
    }

    pub fn root(&self) -> [u8; 32] {
        *self.tree.root()
    }

    pub fn num_keys(&self) -> usize {
        self.keys.len()
    }

    pub fn get_key(&self, index: usize) -> PublicKey {
        if index < self.keys.len() {
            self.keys[index]
        }
        else {
            PublicKey::default()
        }
    }

    pub fn get_path(&self, index: usize) -> Vec<TreeNode> {
        let path = self.tree.prove(index);
        path.iter().map(|bytes| bytes_to_node(*bytes) ).collect()
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn compare_pub_keys(a: &PublicKey, b: &PublicKey) -> std::cmp::Ordering {
    a.to_bytes().cmp(&b.to_bytes())
}