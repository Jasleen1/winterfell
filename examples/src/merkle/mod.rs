use std::{convert::TryFrom, time::Instant};

use log::debug;

use common::errors::VerifierError;
use evaluator::MerkleEvaluator;
use prover::crypto::{hash::rescue_s, MerkleTree};
use prover::{
    crypto::hash::blake3,
    math::field::{BaseElement, FieldElement, StarkField},
    Assertion, ProofOptions, Prover, StarkProof,
};
use trace::generate_trace;
use verifier::Verifier;

use crate::utils::{rescue, to_byte_vec};

use super::Example;

mod evaluator;
mod trace;

type TreeNode = (BaseElement, BaseElement);

// CONSTANTS
// ================================================================================================

const CYCLE_LENGTH: usize = 16;
const NUM_HASH_ROUNDS: usize = 14;
const HASH_STATE_WIDTH: usize = 4;

// RESCUE HASH CHAIN EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(MerkleExample())
}

pub struct MerkleExample();

impl Example for MerkleExample {
    fn prove(
        &self,
        mut tree_depth: usize,
        mut blowup_factor: usize,
        mut num_queries: usize,
        grinding_factor: u32,
    ) -> (StarkProof, Vec<Assertion>) {
        // apply defaults
        if tree_depth == 0 {
            tree_depth = 7;
        }
        if blowup_factor == 0 {
            blowup_factor = 32;
        }
        if num_queries == 0 {
            num_queries = 28;
        }

        // define leaf index and value, such that hash(value) is the leaf
        // at the specified index in the Merkle tree
        let value = (BaseElement::from(42u8), BaseElement::from(43u8));
        let index = (BaseElement::rand().as_u128() % u128::pow(2, tree_depth as u32)) as usize;

        // build Merkle tree of the specified depth
        let now = Instant::now();
        let tree = build_merkle_tree(tree_depth, value, index);
        debug!(
            "Built Merkle tree of depth {} in {} ms",
            tree_depth,
            now.elapsed().as_millis(),
        );

        // compute Merkle path form the leaf specified by the index
        let now = Instant::now();
        let path = tree.prove(index).into_iter().map(bytes_to_node).collect();
        debug!(
            "Computed Merkle path from leaf {} to root {} in {} ms",
            index,
            hex::encode(tree.root()),
            now.elapsed().as_millis(),
        );

        // generate the execution trace
        debug!(
            "Generating proof for proving membership in a Merkle tree of depth {}\n\
            ---------------------",
            tree_depth
        );
        let now = Instant::now();
        let trace = generate_trace(value, path, index);
        let trace_length = trace[0].len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace.len(),
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // make sure execution trace ends up with the root of the Merkle tree
        let expected_root = bytes_to_node(*tree.root());
        let actual_root = (trace[0][trace_length - 1], trace[1][trace_length - 1]);
        assert!(
            expected_root == actual_root,
            "execution trace did not terminate with the root of the Merkle tree"
        );

        // instantiate the prover
        let options = ProofOptions::new(num_queries, blowup_factor, grinding_factor, blake3);
        let prover = Prover::<MerkleEvaluator>::new(options);

        // assert that the trace terminates with tree root
        let assertions = vec![
            Assertion::new(0, trace_length - 1, expected_root.0),
            Assertion::new(1, trace_length - 1, expected_root.1),
        ];

        // generate the proof and return it together with the assertions
        (prover.prove(trace, assertions.clone()).unwrap(), assertions)
    }

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, VerifierError> {
        let verifier = Verifier::<MerkleEvaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_merkle_tree(depth: usize, value: TreeNode, index: usize) -> MerkleTree {
    let num_leaves = usize::pow(2, depth as u32);
    let leaf_elements = BaseElement::prng_vector([1; 32], num_leaves * 2);
    let mut leaves = Vec::new();
    for i in (0..leaf_elements.len()).step_by(2) {
        leaves.push(node_to_bytes((leaf_elements[i], leaf_elements[i + 1])));
    }

    let mut value_bytes = [0; 32];
    rescue_s(&node_to_bytes(value), &mut value_bytes);
    leaves[index] = value_bytes;

    MerkleTree::new(leaves, rescue_s)
}

fn node_to_bytes(node: TreeNode) -> [u8; 32] {
    let mut result = [0; 32];
    result.copy_from_slice(&to_byte_vec(&[node.0, node.1]));
    result
}

fn bytes_to_node(bytes: [u8; 32]) -> TreeNode {
    let v1 = BaseElement::try_from(&bytes[..16]).unwrap();
    let v2 = BaseElement::try_from(&bytes[16..]).unwrap();
    (v1, v2)
}
