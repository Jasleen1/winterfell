use common::{errors::VerifierError, FieldExtension};
use prover::{Assertions, StarkProof};

pub mod anon;
pub mod fibonacci;
pub mod lamport;
pub mod merkle;
pub mod rescue;
pub mod utils;

#[cfg(test)]
mod tests;

// TYPES AND INTERFACES
// ================================================================================================

pub trait Example {
    fn prepare(
        &mut self,
        n: usize,
        blowup_factor: usize,
        num_queries: usize,
        grinding_factor: u32,
        field_extension: FieldExtension,
    ) -> Assertions;
    fn prove(&self, assertions: Assertions) -> StarkProof;
    fn verify(&self, proof: StarkProof, assertions: Assertions) -> Result<(), VerifierError>;
}
