use crate::{channel::ProverChannel, tests::build_proof_context};
use common::{ComputationContext, PublicCoin};
use crypto::{hash::blake3, MerkleTree, RandomElementGenerator};
use fri::{utils as fri_utils, FriProof, FriProofLayer, PublicCoin as FriPublicCoin};
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
    quartic,
};

use super::{Prover, FOLDING_FACTOR};

// TESTS
// ================================================================================================

#[test]
fn fri_prover() {
    let trace_length = 4096;
    let ce_blowup = 2;
    let lde_blowup = 8;
    let context = build_proof_context(trace_length, ce_blowup, lde_blowup);
    let evaluations = build_evaluations(trace_length, ce_blowup, lde_blowup);

    // compute the proof using distributable FRI algorithm
    let mut prover = Prover::new(&context, &evaluations);

    let mut channel = ProverChannel::new(&context);
    prover.build_layers(&mut channel);

    channel.grind_query_seed();
    let positions = channel.draw_query_positions();

    let proof = prover.build_proof(&positions);

    // compute the proof using original FRI algorithm
    let orig_proof = build_proof_orig(
        &context,
        evaluations,
        channel.fri_layer_commitments(),
        prover.num_partitions(),
        &positions,
    );

    // make sure the proofs are the same
    assert_eq!(orig_proof.layers.len(), proof.layers.len());
    for i in 0..proof.layers.len() {
        assert_eq!(orig_proof.layers[i].depth, proof.layers[i].depth);
        assert_eq!(orig_proof.layers[i].values, proof.layers[i].values);
    }
    assert_eq!(orig_proof.rem_values, proof.rem_values);

    assert_eq!(orig_proof, proof);
}

// ORIGINAL FRI
// ================================================================================================

fn build_proof_orig<E: FieldElement + From<BaseElement>>(
    context: &ComputationContext,
    evaluations: Vec<E>,
    fri_roots: &[[u8; 32]],
    num_partitions: usize,
    positions: &[usize],
) -> FriProof {
    // commit phase -------------------------------------------------------------------------------
    let mut alphas = Vec::new();
    for seed in fri_roots.iter() {
        let mut g = RandomElementGenerator::new(*seed, 0, blake3);
        alphas.push(g.draw::<E>());
    }

    let g = BaseElement::get_root_of_unity(evaluations.len().trailing_zeros());
    let domain = BaseElement::get_power_series(g, evaluations.len());
    let (mut trees, mut values) = build_layers_orig(context, &alphas, evaluations, &domain);

    // shuffle evaluations to match the order in the distributable FRI
    for i in 0..trees.len() - 2 {
        values[i] = shuffle_evaluations(&values[i], num_partitions);
        let hashed_evaluations = fri_utils::hash_values(&values[i], blake3);
        trees[i] = MerkleTree::new(hashed_evaluations, blake3);
    }

    // query phase --------------------------------------------------------------------------------

    let mut positions = positions.to_vec();
    let mut domain_size = trees[0].leaves().len() * FOLDING_FACTOR;

    let mut layers = Vec::with_capacity(trees.len());
    for i in 0..(trees.len() - 1) {
        positions = fri_utils::get_augmented_positions(&positions, domain_size);

        let tree = &trees[i];
        // map positions to their equivalent in distributable FRI tree
        let local_positions = map_positions(&positions, domain_size, num_partitions);
        let proof = tree.prove_batch(&local_positions);

        let mut queried_values: Vec<[E; FOLDING_FACTOR]> =
            Vec::with_capacity(local_positions.len());
        for &position in local_positions.iter() {
            queried_values.push(values[i][position]);
        }

        layers.push(FriProofLayer {
            values: queried_values
                .into_iter()
                .map(|v| E::write_into_vec(&v))
                .collect(),
            paths: proof.nodes,
            depth: proof.depth,
        });
        domain_size /= FOLDING_FACTOR;
    }

    let last_values = &values[values.len() - 1];
    let n = last_values.len();
    let mut remainder = vec![E::ZERO; n * FOLDING_FACTOR];
    for i in 0..last_values.len() {
        remainder[i] = last_values[i][0];
        remainder[i + n] = last_values[i][1];
        remainder[i + n * 2] = last_values[i][2];
        remainder[i + n * 3] = last_values[i][3];
    }

    FriProof {
        layers,
        rem_values: E::write_into_vec(&remainder),
    }
}

pub fn build_layers_orig<E: FieldElement + From<BaseElement>>(
    context: &ComputationContext,
    alphas: &[E],
    mut evaluations: Vec<E>,
    domain: &[BaseElement],
) -> (Vec<MerkleTree>, Vec<Vec<[E; FOLDING_FACTOR]>>) {
    let hash_fn = context.options().hash_fn();

    let mut tree_results: Vec<MerkleTree> = Vec::new();
    let mut value_results: Vec<Vec<[E; FOLDING_FACTOR]>> = Vec::new();

    for depth in 0..context.num_fri_layers() + 1 {
        let transposed_evaluations = quartic::transpose(&evaluations, 1);
        let hashed_evaluations = fri_utils::hash_values(&transposed_evaluations, hash_fn);
        let evaluation_tree = MerkleTree::new(hashed_evaluations, hash_fn);

        evaluations = apply_drp(&transposed_evaluations, domain, depth, alphas[depth]);

        tree_results.push(evaluation_tree);
        value_results.push(transposed_evaluations);
    }

    (tree_results, value_results)
}

fn apply_drp<E: FieldElement + From<BaseElement>>(
    evaluations: &[[E; FOLDING_FACTOR]],
    domain: &[BaseElement],
    depth: usize,
    alpha: E,
) -> Vec<E> {
    let domain_stride = usize::pow(FOLDING_FACTOR, depth as u32);
    let xs = quartic::transpose(domain, domain_stride);

    let polys = quartic::interpolate_batch(&xs, &evaluations);

    quartic::evaluate_batch(&polys, alpha)
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_evaluations(trace_length: usize, ce_blowup: usize, lde_blowup: usize) -> Vec<BaseElement> {
    let len = (trace_length * ce_blowup) as u128;
    let mut p = (0..len).map(BaseElement::new).collect::<Vec<_>>();
    let domain_size = trace_length * lde_blowup;
    p.resize(domain_size, BaseElement::ZERO);

    let g = BaseElement::get_root_of_unity(domain_size.trailing_zeros());
    let twiddles = fft::get_twiddles(g, domain_size);

    fft::evaluate_poly(&mut p, &twiddles, true);
    p
}

fn shuffle_evaluations<E: FieldElement>(
    evaluations: &[[E; 4]],
    num_partitions: usize,
) -> Vec<[E; 4]> {
    let partition_size = evaluations.len() / num_partitions;
    let mut result = Vec::new();
    for i in 0..num_partitions {
        for j in 0..partition_size {
            result.push(evaluations[i + j * num_partitions]);
        }
    }
    result
}

fn map_positions(positions: &[usize], num_evaluations: usize, num_partitions: usize) -> Vec<usize> {
    let local_bits = (num_evaluations / 4).trailing_zeros() - num_partitions.trailing_zeros();
    let mut result = Vec::new();
    for &p in positions.iter() {
        let p_idx = p % num_partitions;
        let loc_p = (p - p_idx) / num_partitions;
        result.push((p_idx << local_bits) | loc_p);
    }
    result.sort();
    result
}
