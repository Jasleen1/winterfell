use super::Example;
use crate::utils::{bytes_to_node, node_to_bytes, rescue, TreeNode};
use common::{errors::VerifierError, FieldExtension};
use log::debug;
use prover::{
    crypto::{
        hash::{blake3, rescue_s},
        MerkleTree,
    },
    math::field::{BaseElement, FieldElement, StarkField},
    Assertions, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::Verifier;

mod trace;
use trace::generate_trace;

mod evaluator;
use evaluator::AnonTokenEvaluator;

// CONSTANTS
// ================================================================================================

const CYCLE_LENGTH: usize = 16;
const NUM_HASH_ROUNDS: usize = 14;
const HASH_STATE_WIDTH: usize = 4;
const TRACE_TABLE_WIDTH: usize = 9;

// ANONYMOUS TOKEN EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(AnonTokenExample {
        options: None,
        service_uuid: BaseElement::rand(),
        token_seed: BaseElement::rand(),
        token_index: 0,
        path: Vec::new(),
    })
}

pub struct AnonTokenExample {
    options: Option<ProofOptions>,
    service_uuid: BaseElement,
    token_seed: BaseElement,
    token_index: usize,
    path: Vec<TreeNode>,
}

impl Example for AnonTokenExample {
    fn prepare(
        &mut self,
        mut tree_depth: usize,
        blowup_factor: usize,
        num_queries: usize,
        grinding_factor: u32,
        field_extension: FieldExtension,
    ) -> Assertions {
        self.options =
            build_proof_options(blowup_factor, num_queries, grinding_factor, field_extension);
        if tree_depth == 0 {
            tree_depth = 7;
        }

        // print out sample values of token seed and service uuid
        debug!(
            "Set token_seed to {:x} and service_uuid to {:x}",
            self.token_seed.as_u128(),
            self.service_uuid.as_u128()
        );

        // compute issued token and service subtoken
        let issued_token = build_issued_token(self.token_seed);
        let subtoken = build_subtoken(self.token_seed, self.service_uuid);
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
        self.token_index =
            (BaseElement::rand().as_u128() % u128::pow(2, tree_depth as u32)) as usize;
        let tree = build_merkle_tree(tree_depth, issued_token, self.token_index);
        debug!(
            "Inserted issued_token into Merkle tree of depth {} at index {} in {} ms",
            tree_depth,
            self.token_index,
            now.elapsed().as_millis(),
        );

        // compute Merkle path from the leaf specified by the index
        let now = Instant::now();
        self.path = tree
            .prove(self.token_index)
            .into_iter()
            .map(bytes_to_node)
            .collect();
        debug!(
            "Computed Merkle path from issued_token to Merkle root {} in {} ms",
            hex::encode(tree.root()),
            now.elapsed().as_millis(),
        );

        // assert that:
        // - the trace terminates with Merkle tree root in registers [1, 2]
        // - registers [5, 6] at step 14 contain value of the subtoken
        // - service_uuid was inserted into register 6 at the first step
        let last_step = ((tree_depth + 1) * 16) - 1;
        let root = BaseElement::read_to_vec(tree.root()).unwrap();
        let mut assertions = Assertions::new(TRACE_TABLE_WIDTH, last_step + 1).unwrap();
        assertions.add_single(1, last_step, root[0]).unwrap();
        assertions.add_single(2, last_step, root[1]).unwrap();
        assertions.add_single(6, 0, self.service_uuid).unwrap();
        assertions.add_single(5, 14, subtoken.0).unwrap();
        assertions.add_single(6, 14, subtoken.1).unwrap();

        assertions
    }

    fn prove(&self, assertions: Assertions) -> StarkProof {
        // generate the execution trace
        debug!("Generating anonymous subtoken proof\n---------------------");
        let now = Instant::now();
        let trace = generate_trace(
            self.token_seed,
            self.token_index,
            self.service_uuid,
            &self.path,
        );
        let trace_length = trace[0].len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace.len(),
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // generate the proof
        let prover = Prover::<AnonTokenEvaluator>::new(self.options.clone().unwrap());
        prover.prove(trace, assertions).unwrap()
    }

    fn verify(&self, proof: StarkProof, assertions: Assertions) -> Result<(), VerifierError> {
        let verifier = Verifier::<AnonTokenEvaluator>::new();
        verifier.verify(proof, assertions)
    }
}

// HELPER FUNCTIONS
// ================================================================================================
#[allow(clippy::unnecessary_wraps)]
fn build_proof_options(
    mut blowup_factor: usize,
    mut num_queries: usize,
    grinding_factor: u32,
    field_extension: FieldExtension,
) -> Option<ProofOptions> {
    if blowup_factor == 0 {
        blowup_factor = 32;
    }
    if num_queries == 0 {
        num_queries = 28;
    }
    let options = ProofOptions::new(
        num_queries,
        blowup_factor,
        grinding_factor,
        blake3,
        field_extension,
    );
    Some(options)
}

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
