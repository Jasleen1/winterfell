use common::errors::VerifierError;
use prover::{Assertion, StarkProof};

pub mod anon;
pub mod fibonacci;
pub mod lamport;
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

pub mod tests {
    use common::Assertion;
    use math::field::{BaseElement, FieldElement};

    use crate::Example;

    pub fn test_basic_proof_verification(
        mut e: Box<dyn Example>,
        this_size: Option<usize>,
        this_blowup_factor: Option<usize>,
        this_num_queries: Option<usize>,
        this_grinding_factor: Option<u32>,
    ) {
        let size = this_size.unwrap_or(16);
        let blowup_factor = this_blowup_factor.unwrap_or(8);
        let num_queries = this_num_queries.unwrap_or(32);
        let grinding_factor = this_grinding_factor.unwrap_or(0);
        let assertions = e.prepare(size, blowup_factor, num_queries, grinding_factor);
        let proof = e.prove(assertions.clone());
        let verified = e.verify(proof, assertions);
        assert_eq!(true, verified.unwrap());
    }

    pub fn test_basic_proof_verification_fail(
        mut e: Box<dyn Example>,
        this_size: Option<usize>,
        this_blowup_factor: Option<usize>,
        this_num_queries: Option<usize>,
        this_grinding_factor: Option<u32>,
    ) {
        let size = this_size.unwrap_or(16);
        let blowup_factor = this_blowup_factor.unwrap_or(8);
        let num_queries = this_num_queries.unwrap_or(32);
        let grinding_factor = this_grinding_factor.unwrap_or(0);
        let proof_assertions = e.prepare(size, blowup_factor, num_queries, grinding_factor);
        let proof = e.prove(proof_assertions.clone());
        let mut fail_assertions: Vec<Assertion> = proof_assertions;
        let mut fail_assertions_last: Assertion = fail_assertions.pop().unwrap();
        let mut last_val: BaseElement = fail_assertions_last.value();
        let one_elt = BaseElement::ONE;
        last_val = last_val + one_elt;
        fail_assertions_last = Assertion::new(
            fail_assertions_last.register(),
            fail_assertions_last.step(),
            last_val,
        );
        fail_assertions.push(fail_assertions_last);
        let verified = e.verify(proof, fail_assertions);
        assert!(verified.is_err());
    }
}
