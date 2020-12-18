#[test]
fn fib2_test_basic_proof_verification() {
    let fib = crate::fibonacci::fib2::get_example();
    crate::tests::test_basic_proof_verification(fib, Some(16), Some(8), Some(32), Some(0));
}

#[test]
fn fib2_test_basic_proof_verification_fail() {
    let fib = crate::fibonacci::fib2::get_example();
    crate::tests::test_basic_proof_verification_fail(fib, Some(16), Some(8), Some(32), Some(0));
}
