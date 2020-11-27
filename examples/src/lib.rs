use common::errors::VerifierError;
use prover::{Assertion, StarkProof};

pub mod anon;
pub mod fibonacci;
pub mod merkle;
pub mod rescue;
pub mod utils;

// TYPES AND INTERFACES
// ================================================================================================

pub trait Example {
    fn prepare(
        &mut self,
        n: usize,
        blowup_factor: usize,
        num_queries: usize,
        grinding_factor: u32,
    ) -> Vec<Assertion>;
    fn prove(&self, assertions: Vec<Assertion>) -> StarkProof;
    fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, VerifierError>;
}
