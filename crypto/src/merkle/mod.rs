use std::slice;
use std::collections::{ HashMap, BTreeSet };
use serde::{ Serialize, Deserialize };
use crate::HashFunction;

#[cfg(test)]
mod tests;

// TYPES AND INTERFACES
// ================================================================================================
pub struct MerkleTree {
    nodes   : Vec<[u8; 32]>,
    values  : Vec<[u8; 32]>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMerkleProof {
    pub values  : Vec<[u8; 32]>,
    pub nodes   : Vec<Vec<[u8; 32]>>,
    pub depth   : u8
}

// MERKLE TREE IMPLEMENTATION
// ================================================================================================
impl MerkleTree {

    /// Creates a new merkle tree from the provide leaves and using the provided hash function.
    pub fn new(leaves: Vec<[u8; 32]>, hash: HashFunction) -> MerkleTree {
        assert!(leaves.len().is_power_of_two(), "number of leaves must be a power of 2");
        assert!(leaves.len() >= 2, "a tree must contain at least 2 leaves");

        let nodes = build_merkle_nodes(&leaves, hash);
        return MerkleTree {
            values  : leaves,
            nodes   : nodes
        };
    }

    /// Returns the root of the tree
    pub fn root(&self) -> &[u8; 32] {
        return &self.nodes[1];
    }

    /// Returns leaf nodes of the tree
    pub fn leaves(&self) -> &[[u8; 32]] {
        return &self.values;
    }

    /// Computes merkle path the given leaf index.
    pub fn prove(&self, index: usize) -> Vec<[u8; 32]> {
        assert!(index < self.values.len(), "invalid index {}", index);

        let mut proof = Vec::new();
        proof.push(self.values[index]);
        proof.push(self.values[index ^ 1]);

        let mut index = (index + self.nodes.len()) >> 1;
        while index > 1 {
            proof.push(self.nodes[index ^ 1]);
            index = index >> 1;
        }

        return proof;
    }

    /// Computes merkle paths for the provided indexes and compresses the paths into a single proof.
    pub fn prove_batch(&self, indexes: &[usize]) -> BatchMerkleProof {
        let n = self.values.len();

        let index_map = map_indexes(indexes, n);
        let indexes = normalize_indexes(indexes);
        let mut values = vec![[0u8; 32]; index_map.len()];
        let mut nodes: Vec<Vec<[u8; 32]>> = Vec::with_capacity(indexes.len());

        // populate the proof with leaf node values
        let mut next_indexes: Vec<usize> = Vec::new();
        for index in indexes {
            let v1 = self.values[index];
            let v2 = self.values[index + 1];

            // only values for indexes that were explicitly requested are included in values array
            let input_index1 = index_map.get(&index);
            let input_index2 = index_map.get(&(index + 1));
            if input_index1.is_some() {
                if input_index2.is_some() {
                    values[*input_index1.unwrap()] = v1;
                    values[*input_index2.unwrap()] = v2;
                    nodes.push(Vec::new());
                }
                else {
                    values[*input_index1.unwrap()] = v1;
                    nodes.push(vec![v2]);
                }
            }
            else {
                values[*input_index2.unwrap()] = v2;
                nodes.push(vec![v1]);
            }

            next_indexes.push((index + n) >> 1);
        }

        // add required internal nodes to the proof, skipping redundancies
        let depth = self.values.len().trailing_zeros() as u8;
        for _ in 1..depth {
            let indexes = next_indexes.clone();
            next_indexes.truncate(0);

            let mut i = 0;
            while i < indexes.len() {
                let sibling_index = indexes[i] ^ 1;
                if i + 1 < indexes.len() && indexes[i + 1] == sibling_index {
                    i += 1;
                }
                else {
                    nodes[i].push(self.nodes[sibling_index]);
                }

                // add parent index to the set of next indexes
                next_indexes.push(sibling_index >> 1);

                i += 1;
            }
        }

        return BatchMerkleProof { values, nodes, depth };
    }

    /// Checks whether the path for the specified index is valid.
    pub fn verify(root: &[u8; 32], index: usize, proof: &[[u8; 32]], hash: HashFunction) -> bool {
        let mut buf = [0u8; 64];
        let mut v = [0u8; 32];

        let r = index & 1;
        &buf[..32].copy_from_slice(&proof[r]);
        &buf[32..64].copy_from_slice(&proof[1 - r]);
        hash(&buf, &mut v);

        let mut index = (index + usize::pow(2, (proof.len() - 1) as u32)) >> 1;
        for i in 2..proof.len() {
            if index & 1 == 0 {
                &buf[..32].copy_from_slice(&v);
                &buf[32..64].copy_from_slice(&proof[i]);
            }
            else {
                &buf[..32].copy_from_slice(&proof[i]);
                &buf[32..64].copy_from_slice(&v);
            }
            hash(&buf, &mut v);
            index = index >> 1;
        }

        return v == *root;
    }

    /// Checks whether the batch proof contains merkle paths for the of the specified indexes.
    pub fn verify_batch(root: &[u8; 32], indexes: &[usize], proof: &BatchMerkleProof, hash: HashFunction) -> bool {
        let mut buf = [0u8; 64];
        let mut v: HashMap<usize, [u8; 32]> = HashMap::new();

        // replace odd indexes, offset, and sort in ascending order
        let offset = usize::pow(2, proof.depth as u32);
        let index_map = map_indexes(indexes, offset - 1);
        let indexes = normalize_indexes(indexes);
        if indexes.len() != proof.nodes.len() { return false; }

        // for each index use values to compute parent nodes
        let mut next_indexes: Vec<usize> = Vec::new();
        let mut proof_pointers: Vec<usize> = Vec::with_capacity(indexes.len());
        for (i, index) in indexes.into_iter().enumerate() {
            // copy values of leaf sibling leaf nodes into the buffer
            match index_map.get(&index) {
                Some(&index1) => {
                    if proof.values.len() <= index1 { return false }
                    &buf[..32].copy_from_slice(&proof.values[index1]);
                    match index_map.get(&(index + 1)) {
                        Some(&index2) => {
                            if proof.values.len() <= index2 { return false }
                            &buf[32..64].copy_from_slice(&proof.values[index2]);
                            proof_pointers.push(0);
                        },
                        None => {
                            if proof.nodes[i].len() < 1 { return false }
                            &buf[32..64].copy_from_slice(&proof.nodes[i][0]);
                            proof_pointers.push(1);
                        }
                    }
                },
                None => {
                    if proof.nodes[i].len() < 1 { return false }
                    &buf[..32].copy_from_slice(&proof.nodes[i][0]);
                    match index_map.get(&(index + 1)) {
                        Some(&index2) => {
                            if proof.values.len() <= index2 { return false }
                            &buf[32..64].copy_from_slice(&proof.values[index2]);
                        },
                        None => return false
                    }
                    proof_pointers.push(1);
                }
            }

            // hash sibling nodes into their parent
            let mut parent = [0u8; 32];
            hash(&buf, &mut parent);

            let parent_index = offset + index >> 1;
            v.insert(parent_index, parent);
            next_indexes.push(parent_index);
        }

        // iteratively move up, until we get to the root
        for _ in 1..proof.depth {
            let indexes = next_indexes.clone();
            next_indexes.truncate(0);

            let mut i = 0;
            while i < indexes.len() {
                let node_index = indexes[i];
                let sibling_index = node_index ^ 1;

                // determine the sibling
                let sibling: &[u8; 32];
                if i + 1 < indexes.len() && indexes[i + 1] == sibling_index {
                    sibling = match v.get(&sibling_index) {
                        Some(sibling) => sibling,
                        None => return false
                    };
                    i += 1;
                }
                else {
                    let pointer = proof_pointers[i];
                    if proof.nodes[i].len() <= pointer { return false }
                    sibling = &proof.nodes[i][pointer];
                    proof_pointers[i] += 1;
                }

                // get the node from the map of hashed nodes
                let node = match v.get(&node_index) {
                    Some(node) => node,
                    None => return false
                };

                // compute parent node from node and sibling
                if node_index & 1 != 0 {
                    &buf[..32].copy_from_slice(sibling);
                    &buf[32..64].copy_from_slice(node);
                }
                else {
                    &buf[..32].copy_from_slice(node);
                    &buf[32..64].copy_from_slice(sibling);
                }
                let mut parent = [0u8; 32];
                hash(&buf, &mut parent);

                // add the parent node to the next set of nodes
                let parent_index = node_index >> 1;
                v.insert(parent_index, parent);
                next_indexes.push(parent_index);

                i += 1;
            }
        }

        return *root == *v.get(&1).unwrap();
    }
}

// HELPER FUNCTIONS
// ================================================================================================

pub fn build_merkle_nodes(leaves: &[[u8; 32]], hash: HashFunction) -> Vec<[u8; 32]> {
    let n = leaves.len() / 2;

    // create un-initialized array to hold all intermediate nodes
    let mut nodes: Vec<[u8; 32]> = Vec::with_capacity(2 * n);
    unsafe { nodes.set_len(2 * n); }
    nodes[0] = [0u8; 32];

    // re-interpret leaves as an array of two leaves fused together
    let two_leaves = unsafe { slice::from_raw_parts(leaves.as_ptr() as *const [u8; 64], n) };

    // build first row of internal nodes (parents of leaves)
    for (i, j) in (0..n).zip(n..nodes.len()) {
        hash(&two_leaves[i], &mut nodes[j]);
    }

    // re-interpret nodes as an array of two nodes fused together
    let two_nodes = unsafe { slice::from_raw_parts(nodes.as_ptr() as *const [u8; 64], n) };

    // calculate all other tree nodes
    for i in (1..n).rev() {
        hash(&two_nodes[i], &mut nodes[i]);
    }

    return nodes;
}

fn map_indexes(indexes: &[usize], max_valid: usize) -> HashMap<usize, usize> {
    let mut map = HashMap::new();
    for (i, index) in indexes.iter().cloned().enumerate() {
        map.insert(index, i);
        assert!(index <= max_valid, "invalid index {}", index);
    }
    assert!(indexes.len() == map.len(), "repeating indexes detected");
    return map;
}

fn normalize_indexes(indexes: &[usize]) -> Vec<usize> {
    let mut set = BTreeSet::new();
    for &index in indexes {
        set.insert(index - (index & 1));
    }
    return set.into_iter().collect();
}
