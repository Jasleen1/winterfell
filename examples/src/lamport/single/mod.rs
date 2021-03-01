use super::{
    message_to_elements, rescue, Example, PrivateKey, Signature, CYCLE_LENGTH, NUM_HASH_ROUNDS,
};
use common::errors::VerifierError;
use log::debug;
use prover::{
    crypto::hash::blake3,
    math::field::{BaseElement, FieldElement},
    Assertions, ProofOptions, Prover, StarkProof,
};
use std::time::Instant;
use verifier::Verifier;

mod trace;
use trace::generate_trace;

mod evaluator;
use evaluator::LamportPlusEvaluator;

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
    ) -> Assertions {
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
        let mut assertions = Assertions::new(STATE_WIDTH, last_step + 1).unwrap();
        // power of two register is initialized to one
        assertions.add_single(0, 0, BaseElement::ONE).unwrap();
        // message aggregators are initialized to zeros
        assertions.add_single(3, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(4, 0, BaseElement::ZERO).unwrap();
        // last two rate registers and capacity registers are
        // are initialized to zeros
        assertions.add_single(7, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(8, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(9, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(10, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(13, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(14, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(15, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(16, 0, BaseElement::ZERO).unwrap();
        // all public key registers are initialized to zeros
        assertions.add_single(17, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(18, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(19, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(20, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(21, 0, BaseElement::ZERO).unwrap();
        assertions.add_single(22, 0, BaseElement::ZERO).unwrap();
        // last bits of m0 and m1 are 0s
        assertions
            .add_single(1, last_step, BaseElement::ZERO)
            .unwrap();
        assertions
            .add_single(2, last_step, BaseElement::ZERO)
            .unwrap();
        // correct message was used during proof generation
        assertions
            .add_single(3, last_step, self.msg_elements[0])
            .unwrap();
        assertions
            .add_single(4, last_step, self.msg_elements[1])
            .unwrap();
        // correct public key was used during proof generation
        assertions
            .add_single(17, last_step, pk_elements[0])
            .unwrap();
        assertions
            .add_single(18, last_step, pk_elements[1])
            .unwrap();
        assertions
    }

    fn prove(&self, assertions: Assertions) -> StarkProof {
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
