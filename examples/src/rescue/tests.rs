use common::{FieldExtension, ProofOptions};
use prover::crypto::hash;

#[test]
fn rescue_test_basic_proof_verification() {
    let rescue_eg = Box::new(super::RescueExample::new(build_options(false)));
    crate::tests::test_basic_proof_verification(rescue_eg, 128);
}

#[test]
fn rescue_test_basic_proof_verification_extension() {
    let rescue_eg = Box::new(super::RescueExample::new(build_options(true)));
    crate::tests::test_basic_proof_verification(rescue_eg, 128);
}

#[test]
fn rescue_test_basic_proof_verification_fail() {
    let rescue_eg = Box::new(super::RescueExample::new(build_options(false)));
    crate::tests::test_basic_proof_verification_fail(rescue_eg, 128);
}

fn build_options(use_extension_field: bool) -> ProofOptions {
    let extension = if use_extension_field {
        FieldExtension::Quadratic
    } else {
        FieldExtension::None
    };
    ProofOptions::new(32, 16, 0, hash::blake3, extension)
}
