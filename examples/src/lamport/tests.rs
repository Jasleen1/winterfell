#[test]
fn lamport_test_basic_proof_verification() {
    let lamport_eg = crate::lamport::single::get_example();
    crate::tests::test_basic_proof_verification(lamport_eg, Some(2), Some(8), Some(32), Some(0));
}

#[test]
fn lamport_test_basic_proof_verification_fail() {
    let lamport_eg = crate::lamport::single::get_example();
    crate::tests::test_basic_proof_verification_fail(
        lamport_eg,
        Some(128),
        Some(8),
        Some(32),
        Some(0),
    );
}
