use common::errors::VerifierError;
use prover::{Assertion, StarkProof};

pub mod fibonacci;
pub mod rescue;
pub mod utils;

// TYPES AND INTERFACES
// ================================================================================================

pub trait Example {
    fn prove(
        &self,
        n: usize,
        blowup_factor: usize,
        num_queries: usize,
    ) -> (StarkProof, Vec<Assertion>);
    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, VerifierError>;
}
