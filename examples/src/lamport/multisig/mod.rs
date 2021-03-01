use super::{
    message_to_elements, rescue, Example, PrivateKey, Signature, CYCLE_LENGTH, NUM_HASH_ROUNDS,
};
use log::debug;
use prover::{
    crypto::hash::blake3, math::field::BaseElement, Assertions, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::{Verifier, VerifierError};

mod trace;
use trace::generate_trace;

mod evaluator;
use evaluator::LamportPlusEvaluator;

mod assertions;
use assertions::build_assertions;

// CONSTANTS
// ================================================================================================

const STATE_WIDTH: usize = 22;
const SIG_CYCLE_LENGTH: usize = 128 * CYCLE_LENGTH; // 1024 steps

// LAMPORT MULTI-MESSAGE, MULTI-KEY, SIGNATURE EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(LamportMultisigExample {
        options: None,
        signatures: Vec::new(),
        messages: Vec::new(),
    })
}

pub struct LamportMultisigExample {
    options: Option<ProofOptions>,
    signatures: Vec<Signature>,
    messages: Vec<[BaseElement; 2]>,
}

impl Example for LamportMultisigExample {
    fn prepare(
        &mut self,
        mut num_signatures: usize,
        blowup_factor: usize,
        num_queries: usize,
        grinding_factor: u32,
    ) -> Assertions {
        self.options = build_proof_options(blowup_factor, num_queries, grinding_factor);

        // set default value of num_signatures to 1
        if num_signatures == 0 {
            num_signatures = 1;
        }

        // generate private/public key pairs for the specified number of signatures
        let mut private_keys = Vec::with_capacity(num_signatures);
        let mut public_keys = Vec::with_capacity(num_signatures);
        let now = Instant::now();
        for i in 0..num_signatures {
            private_keys.push(PrivateKey::from_seed([i as u8; 32]));
            public_keys.push(private_keys[i].pub_key().to_elements());
        }
        debug!(
            "Generated {} private-public key pairs in {} ms",
            num_signatures,
            now.elapsed().as_millis()
        );

        // sign messages
        let now = Instant::now();
        for (i, private_key) in private_keys.iter().enumerate() {
            let msg = format!("test message {}", i);
            self.signatures.push(private_key.sign(msg.as_bytes()));
            self.messages.push(message_to_elements(msg.as_bytes()));
        }
        debug!(
            "Signed {} messages in {} ms",
            num_signatures,
            now.elapsed().as_millis()
        );

        // verify signature
        let now = Instant::now();
        for (i, signature) in self.signatures.iter().enumerate() {
            let pk = private_keys[i].pub_key();
            let msg = format!("test message {}", i);
            assert_eq!(true, pk.verify(msg.as_bytes(), &signature));
        }
        debug!(
            "Verified {} signature in {} ms",
            num_signatures,
            now.elapsed().as_millis()
        );

        // build assertions for the computation
        build_assertions(&self.messages, &public_keys)
    }

    fn prove(&self, assertions: Assertions) -> StarkProof {
        // generate the execution trace
        debug!(
            "Generating proof for verifying {} Lamport+ signatures \n\
            ---------------------",
            self.signatures.len(),
        );

        let now = Instant::now();
        let trace = generate_trace(&self.messages, &self.signatures);
        let trace_length = trace[0].len();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace.len(),
            trace_length.trailing_zeros(),
            now.elapsed().as_millis()
        );

        // generate the proof
        let prover = Prover::<LamportPlusEvaluator>::new(self.options.clone().unwrap());
        prover.prove(trace, assertions).unwrap()
    }

    fn verify(&self, proof: StarkProof, assertions: Assertions) -> Result<bool, VerifierError> {
        let verifier = Verifier::<LamportPlusEvaluator>::new();
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
) -> Option<ProofOptions> {
    if blowup_factor == 0 {
        blowup_factor = 64;
    }
    if num_queries == 0 {
        num_queries = 28;
    }
    let options = ProofOptions::new(num_queries, blowup_factor, grinding_factor, blake3);
    Some(options)
}
