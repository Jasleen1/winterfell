#[test]
fn rescue_test_basic_proof_verification() {
    let rescue_eg = crate::rescue::get_example();
    crate::tests::test_basic_proof_verification(rescue_eg, Some(128), Some(8), Some(32), Some(0));
}

#[test]
fn rescue_test_basic_proof_verification_fail() {
    let rescue_eg = crate::rescue::get_example();
    crate::tests::test_basic_proof_verification_fail(
        rescue_eg,
        Some(128),
        Some(8),
        Some(32),
        Some(0),
    );
}
