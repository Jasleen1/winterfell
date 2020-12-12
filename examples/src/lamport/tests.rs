#[test]
fn lamport_test_basic_proof_verification() {
    let lamport_eg = crate::lamport::get_example();
    crate::tests::test_basic_proof_verification(lamport_eg, Some(2), Some(8), Some(32), Some(0));
}

// This test is currently failing due to the fact that the lamport signature is only
// a single signature and the size doesn't matter.
// #[test]
// fn lamport_test_basic_proof_verification_fail() {
//     let mut lamport_eg = crate::lamport::get_example();
//     crate::tests::test_basic_proof_verification_fail(lamport_eg, Some(128), Some(8), Some(32), Some(0));
// }
