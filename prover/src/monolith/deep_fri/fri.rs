use super::super::types::LdeDomain;
use crate::channel::ProverChannel;
use common::{
    fri_utils::{get_augmented_positions, hash_values},
    proof::{FriLayer, FriProof},
    ComputationContext, PublicCoin,
};
use core::convert::From;
use crypto::MerkleTree;
use math::{
    field::{BaseElement, FieldElement},
    quartic,
};

// CONSTANTS
// ================================================================================================

const FOLDING_FACTOR: usize = ComputationContext::FRI_FOLDING_FACTOR;
const MAX_REMAINDER_LENGTH: usize = ComputationContext::MAX_FRI_REMAINDER_LENGTH;

// PROCEDURES
// ================================================================================================

/// Executes commit phase of FRI protocol which recursively applies a degree-respecting projection
/// to evaluations of some function F over a larger domain. The degree of the function implied
/// but evaluations is reduced by FOLDING_FACTOR at every step until the remaining evaluations
/// can fit into a vector of at most MAX_REMAINDER_LENGTH. At each layer of recursion the
/// current evaluations are committed to using a Merkle tree, and the root of this tree is used
/// to derive randomness for the subsequent application of degree-respecting projection.
pub fn build_layers<E: FieldElement + From<BaseElement>>(
    context: &ComputationContext,
    channel: &mut ProverChannel,
    mut evaluations: Vec<E>,
    lde_domain: &LdeDomain,
) -> (Vec<MerkleTree>, Vec<Vec<[E; FOLDING_FACTOR]>>) {
    let hash_fn = context.options().hash_fn();

    // TODO: ideally we should use a Merkle tree implementation which allows storing
    // arbitrary-sized values as leaves
    let mut tree_results: Vec<MerkleTree> = Vec::new();
    let mut value_results: Vec<Vec<[E; FOLDING_FACTOR]>> = Vec::new();

    // get a reference to all values of the LDE domain
    let domain = lde_domain.values();

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
        let coeff = channel.draw_fri_point::<E>(depth as usize);
        evaluations = apply_drp(&transposed_evaluations, domain, depth, coeff);

        tree_results.push(evaluation_tree);
        value_results.push(transposed_evaluations);
    }

    // make sure remainder length does not exceed max allowed value
    let remainder_length = value_results[value_results.len() - 1].len() * FOLDING_FACTOR;
    debug_assert!(
        remainder_length <= MAX_REMAINDER_LENGTH,
        "last FRI layer cannot exceed {} elements, but was {} elements",
        MAX_REMAINDER_LENGTH,
        remainder_length
    );

    (tree_results, value_results)
}

/// Executes query phase of FRI protocol. For each of the provided `positions`, corresponding
/// evaluations from each of the layers are recorded into the proof together with Merkle
/// authentication paths from the root of layer commitment trees.
pub fn build_proof<E: FieldElement>(
    trees: Vec<MerkleTree>,
    values: Vec<Vec<[E; FOLDING_FACTOR]>>,
    positions: &[usize],
) -> FriProof {
    let mut positions = positions.to_vec();
    let mut domain_size = trees[0].leaves().len() * FOLDING_FACTOR;

    // for all trees, except the last one, record tree root, authentication paths
    // to row evaluations, and values for row evaluations
    let mut layers = Vec::with_capacity(trees.len());
    for i in 0..(trees.len() - 1) {
        positions = get_augmented_positions(&positions, domain_size);

        let tree = &trees[i];
        let proof = tree.prove_batch(&positions);

        let mut queried_values: Vec<[E; FOLDING_FACTOR]> = Vec::with_capacity(positions.len());
        for &position in positions.iter() {
            queried_values.push(values[i][position]);
        }

        layers.push(FriLayer {
            values: queried_values
                .into_iter()
                .map(|v| E::slice_to_bytes(&v))
                .collect(),
            paths: proof.nodes,
            depth: proof.depth,
        });
        domain_size /= FOLDING_FACTOR;
    }

    // use the remaining polynomial values directly as proof
    // TODO: write remainder to the proof in transposed form?
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
        rem_values: E::slice_to_bytes(&remainder),
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Applies degree-respecting projection to the `evaluations` reducing the degree of evaluations
/// by FOLDING_FACTOR. This is equivalent to the following:
/// - Let `evaluations` contain the evaluations of polynomial f(x) of degree k
/// - Group coefficients of f so that f(x) = a(x) + x * b(x) + x^2 * c(x) + x^3 * d(x)
/// - Compute random linear combination of a, b, c, d as:
///   f'(x) = a + alpha * b + alpha^2 * c + alpha^3 * d, where alpha is a random coefficient
/// - evaluate f'(x) on a domain which consists of x^4 from the original domain (and thus is
///   1/4 the size)
/// note: that to compute an x in the new domain, we need 4 values from the old domain:
/// x^{1/4}, x^{2/4}, x^{3/4}, x
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
