use crate::{
    utils::{bytes_to_node, node_to_bytes, rescue, TreeNode},
    Example, ExampleOptions,
};
use log::debug;
use prover::{
    crypto::{hash::rescue_s, MerkleTree},
    math::field::{BaseElement, FieldElement, StarkField},
    Assertions, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::{Verifier, VerifierError};

mod trace;
use trace::generate_trace;

mod evaluator;
use evaluator::MerkleEvaluator;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

const CYCLE_LENGTH: usize = 16;
const NUM_HASH_ROUNDS: usize = 14;
const HASH_STATE_WIDTH: usize = 4;
const TRACE_WIDTH: usize = 5;

// MERKLE AUTHENTICATION PATH EXAMPLE
// ================================================================================================
pub fn get_example(options: ExampleOptions) -> Box<dyn Example> {
    Box::new(MerkleExample::new(options.to_proof_options(28, 32)))
}

pub struct MerkleExample {
    options: ProofOptions,
    value: TreeNode,
    path: Vec<TreeNode>,
    index: usize,
}

impl MerkleExample {
    pub fn new(options: ProofOptions) -> MerkleExample {
        MerkleExample {
            options,
            value: (BaseElement::from(42u8), BaseElement::from(43u8)),
            path: Vec::new(),
            index: 0,
        }
    }
}

// EXAMPLE IMPLEMENTATION
// ================================================================================================

impl Example for MerkleExample {
    fn prepare(&mut self, tree_depth: usize) -> Assertions {
        assert!(
            (tree_depth + 1).is_power_of_two(),
            "tree depth must be one less than a power of 2"
        );
        self.index = (BaseElement::rand().as_u128() % u128::pow(2, tree_depth as u32)) as usize;

        // build Merkle tree of the specified depth
        let now = Instant::now();
        let tree = build_merkle_tree(tree_depth, self.value, self.index);
        debug!(
            "Built Merkle tree of depth {} in {} ms",
            tree_depth,
            now.elapsed().as_millis(),
        );

        // compute Merkle path form the leaf specified by the index
        let now = Instant::now();
        self.path = tree
            .prove(self.index)
            .into_iter()
            .map(bytes_to_node)
            .collect();
        debug!(
            "Computed Merkle path from leaf {} to root {} in {} ms",
            self.index,
            hex::encode(tree.root()),
            now.elapsed().as_millis(),
        );

        // assert that the trace terminates with tree root
        let root = BaseElement::read_to_vec(tree.root()).unwrap();
        let last_step = ((tree_depth + 1) * 16) - 1;
        let mut assertions = Assertions::new(TRACE_WIDTH, last_step + 1).unwrap();
        assertions.add_single(0, last_step, root[0]).unwrap();
        assertions.add_single(1, last_step, root[1]).unwrap();
        assertions
    }

    fn prove(&self, assertions: Assertions) -> StarkProof {
        // generate the execution trace
        debug!(
            "Generating proof for proving membership in a Merkle tree of depth {}\n\
            ---------------------",
            self.path.len()
        );
        let now = Instant::now();
        let trace = generate_trace(self.value, &self.path, self.index);
        let trace_length = trace[0].len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace.len(),
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // generate the proof
        let prover = Prover::<MerkleEvaluator>::new(self.options.clone());
        prover.prove(trace, assertions).unwrap()
    }

    fn verify(&self, proof: StarkProof, assertions: Assertions) -> Result<(), VerifierError> {
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
