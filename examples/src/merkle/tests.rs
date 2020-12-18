#[test]
fn merkle_test_basic_proof_verification() {
    let merkle = crate::merkle::get_example();
    crate::tests::test_basic_proof_verification(merkle, Some(7), Some(8), Some(32), Some(0));
}

#[test]
fn merkle_test_basic_proof_verification_fail() {
    let merkle = crate::merkle::get_example();
    crate::tests::test_basic_proof_verification_fail(merkle, Some(7), Some(8), Some(32), Some(0));
}
