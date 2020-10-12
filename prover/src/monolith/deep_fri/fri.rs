use super::super::types::LdeDomain;
use crate::channel::ProverChannel;
use common::stark::{
    fri_utils::{get_augmented_positions, hash_values},
    FriLayer, FriProof, ProofContext, PublicCoin,
};
use crypto::MerkleTree;
use math::{
    field::{f128::FieldElement, StarkField},
    quartic,
};
use std::mem;

pub fn reduce(
    context: &ProofContext,
    channel: &mut ProverChannel,
    evaluations: &[FieldElement],
    lde_domain: &LdeDomain,
) -> (Vec<MerkleTree>, Vec<Vec<[FieldElement; 4]>>) {
    let hash_fn = context.options().hash_fn();

    let mut tree_results: Vec<MerkleTree> = Vec::new();
    let mut value_results: Vec<Vec<[FieldElement; 4]>> = Vec::new();

    // transpose evaluations into a matrix with 4 columns and put its rows into a Merkle tree
    let mut p_values = quartic::transpose(evaluations, 1);
    let hashed_values = hash_values(&p_values, hash_fn);
    let mut p_tree = MerkleTree::new(hashed_values, hash_fn);

    let domain = lde_domain.values();

    // reduce the degree by 4 at each iteration until the remaining polynomial is small enough
    for depth in 0..context.num_fri_layers() {
        // commit to the FRI layer
        channel.commit_fri_layer(*p_tree.root());

        // build polynomials from each row of the polynomial value matrix
        let xs = quartic::transpose(domain, usize::pow(4, depth as u32));
        let polys = quartic::interpolate_batch(&xs, &p_values);

        // select a pseudo-random x coordinate and evaluate each row polynomial at that x
        let special_x = channel.draw_fri_point(depth as usize);
        let column = quartic::evaluate_batch(&polys, special_x);

        // break the column in a polynomial value matrix for the next layer
        let mut c_values = quartic::transpose(&column, 1);

        // put the resulting matrix into a Merkle tree
        let hashed_values = hash_values(&c_values, hash_fn);
        let mut c_tree = MerkleTree::new(hashed_values, hash_fn);

        // set p_tree = c_tree and p_values = c_values for the next iteration of the loop
        mem::swap(&mut c_tree, &mut p_tree);
        mem::swap(&mut c_values, &mut p_values);

        // add p_tree and p_values from this loop (which is now under c_tree and c_values) to the result
        tree_results.push(c_tree);
        value_results.push(c_values);
    }

    // commit to the last FRI layer
    channel.commit_fri_layer(*p_tree.root());

    // make sure remainder length does not exceed max allowed value
    debug_assert!(
        p_values.len() * 4 <= ProofContext::MAX_FRI_REMAINDER_LENGTH,
        "last FRI layer cannot exceed {} elements, but was {} elements",
        ProofContext::MAX_FRI_REMAINDER_LENGTH,
        p_values.len() * 4
    );

    // add the tree at the last layer (the remainder) to the result
    tree_results.push(p_tree);
    value_results.push(p_values);

    (tree_results, value_results)
}

pub fn build_proof(
    trees: Vec<MerkleTree>,
    values: Vec<Vec<[FieldElement; 4]>>,
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

        let mut queried_values: Vec<[FieldElement; 4]> = Vec::with_capacity(positions.len());
        for &position in positions.iter() {
            queried_values.push(values[i][position]);
        }

        layers.push(FriLayer {
            values: queried_values,
            paths: proof.nodes,
            depth: proof.depth,
        });
        domain_size /= 4;
    }

    // use the remaining polynomial values directly as proof
    let last_values = &values[values.len() - 1];
    let n = last_values.len();
    let mut remainder = vec![FieldElement::ZERO; n * 4];
    for i in 0..last_values.len() {
        remainder[i] = last_values[i][0];
        remainder[i + n] = last_values[i][1];
        remainder[i + n * 2] = last_values[i][2];
        remainder[i + n * 3] = last_values[i][3];
    }

    FriProof {
        layers,
        rem_values: remainder,
    }
}
