use super::{
    message_to_elements, rescue, Example, PrivateKey, PublicKey, Signature,
    CYCLE_LENGTH as HASH_CYCLE_LENGTH, NUM_HASH_ROUNDS,
};
use crate::utils::TreeNode;
use common::FieldExtension;
use log::debug;
use prover::{
    crypto::hash::blake3,
    math::field::{BaseElement, FieldElement},
    Assertions, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::{Verifier, VerifierError};

mod signature;
use signature::AggPublicKey;

mod assertions;
use assertions::build_assertions;

mod trace;
use trace::generate_trace;

mod evaluator;
use evaluator::LamportThresholdEvaluator;

// CONSTANTS
// ================================================================================================

const STATE_WIDTH: usize = 32;
const SIG_CYCLE_LENGTH: usize = 128 * HASH_CYCLE_LENGTH; // 1024 steps

// LAMPORT THRESHOLD SIGNATURE EXAMPLE
// ================================================================================================

pub fn get_example() -> Box<dyn Example> {
    Box::new(LamportThresholdExample {
        options: None,
        pub_key: AggPublicKey::new(vec![PublicKey::default(); 4]),
        signatures: Vec::new(),
        message: [BaseElement::ZERO; 2],
    })
}

pub struct LamportThresholdExample {
    options: Option<ProofOptions>,
    pub_key: AggPublicKey,
    signatures: Vec<(usize, Signature)>,
    message: [BaseElement; 2],
}

impl Example for LamportThresholdExample {
    fn prepare(
        &mut self,
        mut num_signers: usize,
        blowup_factor: usize,
        num_queries: usize,
        grinding_factor: u32,
        field_extension: FieldExtension,
    ) -> Assertions {
        self.options =
            build_proof_options(blowup_factor, num_queries, grinding_factor, field_extension);

        // set default value of num_signers to 3
        if num_signers == 0 {
            num_signers = 3;
        }

        // generate private/public key pairs for the specified number of signatures
        let now = Instant::now();
        let private_keys = build_keys(num_signers);
        debug!(
            "Generated {} private-public key pairs in {} ms",
            num_signers,
            now.elapsed().as_millis()
        );
        let public_keys = private_keys.iter().map(|k| k.pub_key()).collect();

        // sign the message with the subset of previously generated keys
        let message = "test message";
        self.message = message_to_elements(message.as_bytes());
        let selected_indexes = pick_random_indexes(num_signers);
        for &key_index in selected_indexes.iter() {
            let signature = private_keys[key_index].sign(message.as_bytes());
            self.signatures.push((key_index, signature));
        }

        // build the aggregated public key
        let now = Instant::now();
        self.pub_key = AggPublicKey::new(public_keys);
        debug!(
            "Built aggregated public key in {} ms",
            now.elapsed().as_millis()
        );

        // build and return the assertions
        build_assertions(&self.pub_key, self.message, self.signatures.len())
    }

    fn prove(&self, assertions: Assertions) -> StarkProof {
        // generate the execution trace
        debug!(
            "Generating proof for verifying {}-of-{} signature \n\
            ---------------------",
            self.signatures.len(),
            self.pub_key.num_keys(),
        );

        let now = Instant::now();
        let trace = generate_trace(&self.pub_key, self.message, &self.signatures);
        let trace_length = trace[0].len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace.len(),
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // generate the proof
        let prover = Prover::<LamportThresholdEvaluator>::new(self.options.clone().unwrap());
        prover.prove(trace, assertions).unwrap()
    }

    fn verify(&self, proof: StarkProof, assertions: Assertions) -> Result<(), VerifierError> {
        let verifier = Verifier::<LamportThresholdEvaluator>::new();
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
        blowup_factor = 64;
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

fn build_keys(num_keys: usize) -> Vec<PrivateKey> {
    let mut result = Vec::with_capacity(num_keys);
    for i in 0..num_keys {
        result.push(PrivateKey::from_seed([i as u8; 32]));
    }
    result.sort_by_key(|k| k.pub_key());
    result
}

fn pick_random_indexes(num_keys: usize) -> Vec<usize> {
    let num_selected_keys = num_keys * 2 / 3;
    let mut result = Vec::with_capacity(num_selected_keys);
    // TODO: change to actual random selection
    for i in 0..num_selected_keys {
        result.push(i);
    }
    result
}
