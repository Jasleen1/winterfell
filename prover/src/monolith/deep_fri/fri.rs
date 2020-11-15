use super::super::types::LdeDomain;
use crate::channel::ProverChannel;
use common::{
    fri_utils::{get_augmented_positions, hash_values},
    proof::{FriLayer, FriProof},
    ComputationContext, PublicCoin,
};
use crypto::MerkleTree;
use math::{
    field::{BaseElement, FieldElement},
    quartic,
};

pub fn build_layers(
    context: &ComputationContext,
    channel: &mut ProverChannel,
    mut evaluations: Vec<BaseElement>,
    lde_domain: &LdeDomain,
) -> (Vec<MerkleTree>, Vec<Vec<[BaseElement; 4]>>) {
    let hash_fn = context.options().hash_fn();

    let mut tree_results: Vec<MerkleTree> = Vec::new();
    let mut value_results: Vec<Vec<[BaseElement; 4]>> = Vec::new();

    // reduce the degree by 4 at each iteration until the remaining polynomial is small enough;
    // + 1 is for the remainder
    for depth in 0..context.num_fri_layers() + 1 {
        // commit to the evaluations at the current layer; we do this by first transposing the
        // evaluations into a matrix of 4 columns, and then building a Merkle tree from the
        // rows of this matrix; we do this so that we could de-commit to 4 values with a sing
        // Merkle authentication path.
        let transposed_evaluations = quartic::transpose(&evaluations, 1);
        let hashed_evaluations = hash_values(&transposed_evaluations, hash_fn);
        let evaluation_tree = MerkleTree::new(hashed_evaluations, hash_fn);
        channel.commit_fri_layer(*evaluation_tree.root());

        // draw a pseudo-random coefficient from the channel, and use it in degree-respecting
        // projection to reduce the degree of evaluations by 4
        let coeff = channel.draw_fri_point::<BaseElement>(depth as usize);
        evaluations = apply_drp(&transposed_evaluations, lde_domain.values(), depth, coeff);

        tree_results.push(evaluation_tree);
        value_results.push(transposed_evaluations);
    }

    // make sure remainder length does not exceed max allowed value
    let remainder_length = value_results[value_results.len() - 1].len() * 4;
    debug_assert!(
        remainder_length <= ComputationContext::MAX_FRI_REMAINDER_LENGTH,
        "last FRI layer cannot exceed {} elements, but was {} elements",
        ComputationContext::MAX_FRI_REMAINDER_LENGTH,
        remainder_length
    );

    (tree_results, value_results)
}

pub fn build_proof(
    trees: Vec<MerkleTree>,
    values: Vec<Vec<[BaseElement; 4]>>,
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

        let mut queried_values: Vec<[BaseElement; 4]> = Vec::with_capacity(positions.len());
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
    let mut remainder = vec![BaseElement::ZERO; n * 4];
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

// HELPER FUNCTIONS
// ================================================================================================

/// Applies degree-respecting projection to the `evaluations`
fn apply_drp<E: FieldElement>(
    evaluations: &[[E; 4]],
    domain: &[E],
    depth: usize,
    coeff: E,
) -> Vec<E> {
    let domain_stride = usize::pow(ComputationContext::FRI_FOLDING_FACTOR, depth as u32);
    let xs = quartic::transpose(domain, domain_stride);

    let polys = quartic::interpolate_batch(&xs, &evaluations);

    quartic::evaluate_batch(&polys, coeff)
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {

    use math::{
        fft,
        field::{BaseElement, FieldElement, StarkField},
        polynom, quartic,
    };

    #[test]
    fn apply_drp() {
        // build a polynomial and evaluate it over a larger domain
        let p = BaseElement::prng_vector([1; 32], 1024);
        let mut evaluations = vec![BaseElement::ZERO; p.len() * 8];
        evaluations[..p.len()].copy_from_slice(&p);
        let g = BaseElement::get_root_of_unity(evaluations.len().trailing_zeros());
        let twiddles = fft::get_twiddles(g, evaluations.len());
        fft::evaluate_poly(&mut evaluations, &twiddles, true);

        // apply degree respecting projection
        let coeff = BaseElement::rand();
        let domain = BaseElement::get_power_series(g, evaluations.len());
        let t_evaluations = quartic::transpose(&evaluations, 1);
        evaluations = super::apply_drp(&t_evaluations, &domain, 0, coeff);

        // interpolate evaluations into a polynomial
        let g = BaseElement::get_root_of_unity(evaluations.len().trailing_zeros());
        let inv_twiddles = fft::get_inv_twiddles(g, evaluations.len());
        fft::interpolate_poly(&mut evaluations, &inv_twiddles, true);

        // make sure the degree has been reduced by 4
        assert_eq!(p.len() / 4 - 1, polynom::degree_of(&evaluations));

        // make sure the coefficients of the new polynomial were derived from
        // the original polynomial correctly
        for i in 0..p.len() / 4 {
            let c1 = p[i * 4]
                + p[i * 4 + 1] * coeff
                + p[i * 4 + 2] * coeff.exp(2)
                + p[i * 4 + 3] * coeff.exp(3);
            assert_eq!(c1, evaluations[i]);
        }
    }
}
