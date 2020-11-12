use super::Example;
use crate::utils::{rescue, to_byte_vec};
use common::errors::VerifierError;
use log::debug;
use prover::crypto::{hash::rescue_s, MerkleTree};
use prover::{
    crypto::hash::blake3,
    math::field::{BaseElement, FieldElement, StarkField},
    Assertion, ProofOptions, Prover, StarkProof,
};
use std::{convert::TryFrom, time::Instant};
use verifier::Verifier;

mod trace;
use trace::generate_trace;

mod evaluator;
use evaluator::AnonTokenEvaluator;

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

        // set token seed, service uuid, and token index to sample values
        let token_seed = BaseElement::rand();
        let service_uuid = BaseElement::rand();
        debug!(
            "Set token_seed to {:x} and service_uuid to {:x}",
            token_seed.as_u128(),
            service_uuid.as_u128()
        );

        // compute issued token and service subtoken
        let issued_token = build_issued_token(token_seed);
        let subtoken = build_subtoken(token_seed, service_uuid);
        debug!(
            "Derived issued_token {}",
            hex::encode(node_to_bytes(issued_token))
        );
        debug!(
            "Derived service subtoken {}",
            hex::encode(node_to_bytes(subtoken))
        );

        // build Merkle tree of the specified depth with issued_token located at token_index
        let now = Instant::now();
        let token_index =
            (BaseElement::rand().as_u128() % u128::pow(2, tree_depth as u32)) as usize;
        let tree = build_merkle_tree(tree_depth, issued_token, token_index);
        debug!(
            "Inserted issued_token into Merkle tree of depth {} at index {} in {} ms",
            tree_depth,
            token_index,
            now.elapsed().as_millis(),
        );

        // compute Merkle path from the leaf specified by the index
        let now = Instant::now();
        let path = tree
            .prove(token_index)
            .into_iter()
            .map(bytes_to_node)
            .collect();
        debug!(
            "Computed Merkle path from issued_token to Merkle root {} in {} ms",
            hex::encode(tree.root()),
            now.elapsed().as_millis(),
        );

        // generate the execution trace
        debug!("Generating anonymous subtoken proof\n---------------------");
        let now = Instant::now();
        let trace = generate_trace(token_seed, token_index, service_uuid, path);
        let trace_length = trace[0].len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace.len(),
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // make sure execution trace ends up with the root of the Merkle tree
        let expected_root = bytes_to_node(*tree.root());
        let actual_root = (trace[1][trace_length - 1], trace[2][trace_length - 1]);
        assert!(
            expected_root == actual_root,
            "execution trace did not terminate with the root of the Merkle tree"
        );

        // instantiate the prover
        let options = ProofOptions::new(num_queries, blowup_factor, grinding_factor, blake3);
        let prover = Prover::<AnonTokenEvaluator>::new(options);

        // assert that:
        // - the trace terminates with Merkle tree root in registers [1, 2]
        // - registers [5, 6] at step 14 contain value of the subtoken
        // - service_uuid was inserted into register 6 at the first step
        let assertions = vec![
            Assertion::new(1, trace_length - 1, expected_root.0),
            Assertion::new(2, trace_length - 1, expected_root.1),
            Assertion::new(6, 0, service_uuid),
            Assertion::new(5, 14, subtoken.0),
            Assertion::new(6, 14, subtoken.1),
        ];

        // generate the proof and return it together with the assertions
        (prover.prove(trace, assertions.clone()).unwrap(), assertions)
    }

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, VerifierError> {
        let verifier = Verifier::<AnonTokenEvaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_merkle_tree(depth: usize, issued_token: TreeNode, index: usize) -> MerkleTree {
    let num_leaves = usize::pow(2, depth as u32);
    let leaf_elements = BaseElement::prng_vector([1; 32], num_leaves * 2);
    let mut leaves = Vec::new();
    for i in (0..leaf_elements.len()).step_by(2) {
        leaves.push(node_to_bytes((leaf_elements[i], leaf_elements[i + 1])));
    }
    leaves[index] = node_to_bytes(issued_token);

    MerkleTree::new(leaves, rescue_s)
}

fn build_issued_token(token_seed: BaseElement) -> (BaseElement, BaseElement) {
    let mut result = [0; 32];
    rescue_s(&token_seed.to_bytes(), &mut result);
    bytes_to_node(result)
}

fn build_subtoken(
    token_seed: BaseElement,
    service_uuid: BaseElement,
) -> (BaseElement, BaseElement) {
    let mut result = [0; 32];
    rescue_s(&node_to_bytes((token_seed, service_uuid)), &mut result);
    bytes_to_node(result)
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
