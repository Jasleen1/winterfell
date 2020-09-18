use super::{super::types::LdeDomain, quartic};
use common::{
    stark::{FriLayer, FriProof, ProofOptions},
    utils::{as_bytes, uninit_vector},
};
use crypto::{HashFunction, MerkleTree};
use math::field;
use std::mem;

const MAX_REMAINDER_LENGTH: usize = 256;

pub fn reduce(
    evaluations: &[u128],
    lde_domain: &LdeDomain,
    options: &ProofOptions,
) -> (Vec<MerkleTree>, Vec<Vec<[u128; 4]>>) {
    let mut tree_results: Vec<MerkleTree> = Vec::new();
    let mut value_results: Vec<Vec<[u128; 4]>> = Vec::new();

    // transpose evaluations into a matrix with 4 columns and put its rows into a Merkle tree
    let mut p_values = quartic::transpose(evaluations, 1);
    let hashed_values = hash_values(&p_values, options.hash_fn());
    let mut p_tree = MerkleTree::new(hashed_values, options.hash_fn());

    let domain = lde_domain.values();

    // reduce the degree by 4 at each iteration until the remaining polynomial is small enough
    while p_tree.leaves().len() * 4 > MAX_REMAINDER_LENGTH {
        // build polynomials from each row of the polynomial value matrix
        let depth = tree_results.len() as u32;
        let xs = quartic::transpose(domain, usize::pow(4, depth));
        let polys = quartic::interpolate_batch(&xs, &p_values);

        // select a pseudo-random x coordinate and evaluate each row polynomial at that x
        let special_x = field::prng(*p_tree.root());
        let column = quartic::evaluate_batch(&polys, special_x);

        // break the column in a polynomial value matrix for the next layer
        let mut c_values = quartic::transpose(&column, 1);

        // put the resulting matrix into a Merkle tree
        let hashed_values = hash_values(&c_values, options.hash_fn());
        let mut c_tree = MerkleTree::new(hashed_values, options.hash_fn());

        // set p_tree = c_tree and p_values = c_values for the next iteration of the loop
        mem::swap(&mut c_tree, &mut p_tree);
        mem::swap(&mut c_values, &mut p_values);

        // add p_tree and p_values from this loop (which is now under c_tree and c_values) to the result
        tree_results.push(c_tree);
        value_results.push(c_values);
    }

    // add the tree at the last layer (the remainder)
    tree_results.push(p_tree);
    value_results.push(p_values);

    (tree_results, value_results)
}

pub fn build_proof(
    trees: Vec<MerkleTree>,
    values: Vec<Vec<[u128; 4]>>,
    positions: &[usize],
) -> FriProof {
    let mut positions = positions.to_vec();
    let mut domain_size = trees[0].leaves().len() * 4;

    // for all trees, except the last one, record tree root, authentication paths
    // to row evaluations, and values for row evaluations
    let mut layers = Vec::with_capacity(trees.len());
    for i in 0..(trees.len() - 1) {
        positions = get_augmented_positions(&positions, domain_size);

        let tree = &trees[i];
        let proof = tree.prove_batch(&positions);

        let mut queried_values: Vec<[u128; 4]> = Vec::with_capacity(positions.len());
        for &position in positions.iter() {
            queried_values.push(values[i][position]);
        }

        layers.push(FriLayer {
            root: *tree.root(),
            values: queried_values,
            nodes: proof.nodes,
            depth: proof.depth,
        });
        domain_size /= 4;
    }

    // use the remaining polynomial values directly as proof
    let last_tree = &trees[trees.len() - 1];
    let last_values = &values[values.len() - 1];
    let n = last_values.len();
    let mut remainder = vec![field::ZERO; n * 4];
    for i in 0..last_values.len() {
        remainder[i] = last_values[i][0];
        remainder[i + n] = last_values[i][1];
        remainder[i + n * 2] = last_values[i][2];
        remainder[i + n * 3] = last_values[i][3];
    }

    FriProof {
        layers,
        rem_root: *last_tree.root(),
        rem_values: remainder,
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn get_augmented_positions(positions: &[usize], column_length: usize) -> Vec<usize> {
    let row_length = column_length / 4;
    let mut result = Vec::new();
    for position in positions {
        let ap = position % row_length;
        if !result.contains(&ap) {
            result.push(ap);
        }
    }
    result
}

fn hash_values(values: &[[u128; 4]], hash: HashFunction) -> Vec<[u8; 32]> {
    let mut result: Vec<[u8; 32]> = uninit_vector(values.len());
    for i in 0..values.len() {
        hash(as_bytes(&values[i]), &mut result[i]);
    }
    result
}
