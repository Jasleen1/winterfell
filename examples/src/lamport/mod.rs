use super::Example;
use common::errors::VerifierError;
use log::debug;
use prover::{
    crypto::hash::blake3,
    math::field::{BaseElement, FieldElement},
    Assertion, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::Verifier;

mod signature;
use signature::{message_to_elements, PrivateKey, Signature};

mod rescue;
use rescue::{CYCLE_LENGTH, NUM_ROUNDS as NUM_HASH_ROUNDS};

mod trace;
use trace::generate_trace;

mod evaluator;
use evaluator::LamportPlusEvaluator;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

const STATE_WIDTH: usize = 23;

// LAMPORT SIGNATURE EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(LamportExample {
        options: None,
        signature: Signature {
            ones: vec![],
            zeros: vec![],
        },
        msg_elements: [BaseElement::ZERO; 2],
    })
}

pub struct LamportExample {
    options: Option<ProofOptions>,
    signature: Signature,
    msg_elements: [BaseElement; 2],
}

impl Example for LamportExample {
    fn prepare(
        &mut self,
        _num_signatures: usize,
        blowup_factor: usize,
        num_queries: usize,
        grinding_factor: u32,
    ) -> Vec<Assertion> {
        self.options = build_proof_options(blowup_factor, num_queries, grinding_factor);

        // generate private/public key pair from a seed
        let now = Instant::now();
        let sk = PrivateKey::from_seed([1; 32]);
        debug!(
            "Generated private-public key pair in {} ms",
            now.elapsed().as_millis()
        );

        // sign a message
        let now = Instant::now();
        let msg = "test message 1";
        self.signature = sk.sign(msg.as_bytes());
        debug!(
            "Signed {}-byte message in {} ms",
            msg.as_bytes().len(),
            now.elapsed().as_millis()
        );
        debug!(
            "Signature size: {} bytes",
            bincode::serialize(&self.signature).unwrap().len()
        );

        // verify signature
        let pk = sk.pub_key();
        let now = Instant::now();
        assert_eq!(true, pk.verify(msg.as_bytes(), &self.signature));
        debug!("Verified signature in {} ms", now.elapsed().as_millis());

        self.msg_elements = message_to_elements(msg.as_bytes());

        // assert that the trace terminates with tree root
        let pk_elements = pk.to_elements();
        let last_step = (128 * CYCLE_LENGTH) - 1;
        vec![
            // power of two register is initialized to one
            Assertion::new(0, 0, BaseElement::ONE),
            // message aggregators are initialized to zeros
            Assertion::new(3, 0, BaseElement::ZERO),
            Assertion::new(4, 0, BaseElement::ZERO),
            // last two rate registers and capacity registers are
            // are initialized to zeros
            Assertion::new(7, 0, BaseElement::ZERO),
            Assertion::new(8, 0, BaseElement::ZERO),
            Assertion::new(9, 0, BaseElement::ZERO),
            Assertion::new(10, 0, BaseElement::ZERO),
            Assertion::new(13, 0, BaseElement::ZERO),
            Assertion::new(14, 0, BaseElement::ZERO),
            Assertion::new(15, 0, BaseElement::ZERO),
            Assertion::new(16, 0, BaseElement::ZERO),
            // all public key registers are initialized to zeros
            Assertion::new(17, 0, BaseElement::ZERO),
            Assertion::new(18, 0, BaseElement::ZERO),
            Assertion::new(19, 0, BaseElement::ZERO),
            Assertion::new(20, 0, BaseElement::ZERO),
            Assertion::new(21, 0, BaseElement::ZERO),
            Assertion::new(22, 0, BaseElement::ZERO),
            // last bits of m0 and m1 are 0s
            Assertion::new(1, last_step, BaseElement::ZERO),
            Assertion::new(2, last_step, BaseElement::ZERO),
            // correct message was used during proof generation
            Assertion::new(3, last_step, self.msg_elements[0]),
            Assertion::new(4, last_step, self.msg_elements[1]),
            // correct public key was used during proof generation
            Assertion::new(17, last_step, pk_elements[0]),
            Assertion::new(18, last_step, pk_elements[1]),
        ]
    }

    fn prove(&self, assertions: Vec<Assertion>) -> StarkProof {
        // generate the execution trace
        debug!(
            "Generating proof for verifying Lamport+ signature \n\
            ---------------------"
        );

        let now = Instant::now();
        let trace = generate_trace(&self.msg_elements, &self.signature);
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

    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, VerifierError> {
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
