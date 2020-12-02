use super::Example;
use common::errors::VerifierError;
use log::debug;
use prover::{Assertion, StarkProof};
use std::time::Instant;

mod signature;
use signature::{message_to_elements, PrivateKey, Signature};

mod trace;
use trace::generate_trace;

mod rescue;
use rescue::{CYCLE_LENGTH, NUM_ROUNDS as NUM_HASH_ROUNDS};

// CONSTANTS
// ================================================================================================

const STATE_WIDTH: usize = 23;

// LAMPORT SIGNATURE EXAMPLE
// ================================================================================================
pub fn get_example() -> Box<dyn Example> {
    Box::new(LamportExample())
}

pub struct LamportExample();

impl Example for LamportExample {
    fn prove(
        &self,
        _sequence_length: usize,
        _blowup_factor: usize,
        _num_queries: usize,
        _grinding_factor: u32,
    ) -> (StarkProof, Vec<Assertion>) {
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
        let sig = sk.sign(msg.as_bytes());
        debug!(
            "Signed {}-byte message in {} ms",
            msg.as_bytes().len(),
            now.elapsed().as_millis()
        );
        debug!(
            "Signature size: {} bytes",
            bincode::serialize(&sig).unwrap().len()
        );

        let msg_elements = message_to_elements(msg.as_bytes());
        let trace = generate_trace(&msg_elements, &sig);
        //print_trace(&trace[1..3], 8);

        let trace_length = trace[0].len();
        assert_eq!(msg_elements[0], trace[3][trace_length - 1]);
        assert_eq!(msg_elements[1], trace[4][trace_length - 1]);

        let pk = sk.pub_key();
        let pk_elements = pk.to_elements();
        assert_eq!(pk_elements[0], trace[17][trace_length - 1]);
        assert_eq!(pk_elements[1], trace[18][trace_length - 1]);

        // verify signature
        let now = Instant::now();
        assert_eq!(true, pk.verify(msg.as_bytes(), &sig));
        debug!("Verified signature in {} ms", now.elapsed().as_millis());

        let msg = "test message 2";
        assert_eq!(false, pk.verify(msg.as_bytes(), &sig));

        unimplemented!()
    }

    fn verify(
        &self,
        _proof: StarkProof,
        _assertions: Vec<Assertion>,
    ) -> Result<bool, VerifierError> {
        unimplemented!();
    }
}
