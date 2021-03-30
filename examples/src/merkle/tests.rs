use common::{FieldExtension, ProofOptions};
use prover::crypto::hash;

#[test]
fn merkle_test_basic_proof_verification() {
    let merkle = Box::new(super::MerkleExample::new(build_options(false)));
    crate::tests::test_basic_proof_verification(merkle, 7);
}

#[test]
fn merkle_test_basic_proof_verification_extension() {
    let merkle = Box::new(super::MerkleExample::new(build_options(true)));
    crate::tests::test_basic_proof_verification(merkle, 7);
}

#[test]
fn merkle_test_basic_proof_verification_fail() {
    let merkle = Box::new(super::MerkleExample::new(build_options(false)));
    crate::tests::test_basic_proof_verification_fail(merkle, 7);
}

fn build_options(use_extension_field: bool) -> ProofOptions {
    let extension = if use_extension_field {
        FieldExtension::Quadratic
    } else {
        FieldExtension::None
    };
    ProofOptions::new(32, 16, 0, hash::blake3, extension)
}
