use super::Example;
use common::errors::VerifierError;
use log::debug;
use prover::{Assertion, StarkProof};
use std::time::Instant;

mod signature;
use signature::{gen_keys, sign, verify as verify_sig};

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
        let pk = gen_keys([1; 32]);
        debug!(
            "Generated private-public key pair in {} ms",
            now.elapsed().as_millis()
        );

        // sign a message
        let now = Instant::now();
        let msg = "test message 1";
        let sig = sign(msg.as_bytes(), &pk);
        debug!(
            "Signed {}-byte message in {} ms",
            msg.as_bytes().len(),
            now.elapsed().as_millis()
        );
        debug!(
            "Signature size: {} bytes",
            bincode::serialize(&sig).unwrap().len()
        );

        // verify signature
        let now = Instant::now();
        assert_eq!(true, verify_sig(msg.as_bytes(), pk.pub_key(), &sig));
        debug!("Verified signature in {} ms", now.elapsed().as_millis());

        let msg = "test message 2";
        assert_eq!(false, verify_sig(msg.as_bytes(), pk.pub_key(), &sig));

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
