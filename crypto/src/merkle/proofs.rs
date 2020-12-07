use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchMerkleProof {
    pub values: Vec<[u8; 32]>,
    pub nodes: Vec<Vec<[u8; 32]>>,
    pub depth: u8,
}

impl BatchMerkleProof {
    /// Constructs a batch Merkle proof from individual Merkle authentication paths.
    /// TODO: optimize this to reduce amount of vector cloning.
    pub fn from_paths(paths: &[Vec<[u8; 32]>], indexes: &[usize]) -> BatchMerkleProof {
        assert!(
            paths.len() == indexes.len(),
            "number of paths must equal number of indexes"
        );
        assert!(!paths.is_empty(), "at least one path must be provided");

        let depth = paths[0].len();

        // sort indexes in ascending order, and also re-arrange paths accordingly
        let mut path_map = BTreeMap::new();
        for (&index, path) in indexes.iter().zip(paths.iter().cloned()) {
            path_map.insert(index, path);
        }
        let indexes = path_map.keys().cloned().collect::<Vec<_>>();
        let paths = path_map.values().cloned().collect::<Vec<_>>();
        path_map.clear();

        let mut values = vec![[0u8; 32]; indexes.len()];
        let mut nodes: Vec<Vec<[u8; 32]>> = Vec::with_capacity(indexes.len());

        // populate values and the first layer of proof nodes
        let mut i = 0;
        while i < indexes.len() {
            values[i] = paths[i][0];
            if indexes.len() > i + 1 && are_siblings(indexes[i], indexes[i + 1]) {
                values[i + 1] = paths[i][1];
                nodes.push(vec![]);
                i += 1;
            } else {
                nodes.push(vec![paths[i][1]]);
            }
            path_map.insert(indexes[i] >> 1, paths[i].clone());
            i += 1;
        }

        // populate all remaining layers of proof nodes
        for d in 2..depth {
            let indexes = path_map.keys().cloned().collect::<Vec<_>>();
            let mut next_path_map = BTreeMap::new();

            let mut i = 0;
            while i < indexes.len() {
                let index = indexes[i];
                let path = path_map.get(&index).unwrap();
                if indexes.len() > i + 1 && are_siblings(index, indexes[i + 1]) {
                    i += 1;
                } else {
                    nodes[i].push(path[d]);
                }
                next_path_map.insert(index >> 1, path.clone());
                i += 1;
            }

            std::mem::swap(&mut path_map, &mut next_path_map);
        }

        BatchMerkleProof {
            values,
            nodes,
            depth: (depth - 1) as u8,
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Two nodes are siblings if index of the left node is even and right node
/// immediately follows the left node.
fn are_siblings(left: usize, right: usize) -> bool {
    left & 1 == 0 && right - 1 == left
}
