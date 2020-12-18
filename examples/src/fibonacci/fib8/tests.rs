#[test]
fn fib8_test_basic_proof_verification() {
    let fib = crate::fibonacci::fib8::get_example();
    crate::tests::test_basic_proof_verification(fib, Some(64), Some(8), Some(32), Some(0));
}

#[test]
fn fib8_test_basic_proof_verification_fail() {
    let fib = crate::fibonacci::fib8::get_example();
    crate::tests::test_basic_proof_verification_fail(fib, Some(64), Some(8), Some(32), Some(0));
}
