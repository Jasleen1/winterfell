use super::super::utils::build_proof_options;

#[test]
fn mulfib2_test_basic_proof_verification() {
    let fib = Box::new(super::MulFib2Example::new(build_proof_options(false)));
    crate::tests::test_basic_proof_verification(fib, 16);
}

#[test]
fn mulfib2_test_basic_proof_verification_extension() {
    let fib = Box::new(super::MulFib2Example::new(build_proof_options(true)));
    crate::tests::test_basic_proof_verification(fib, 16);
}

#[test]
fn mulfib2_test_basic_proof_verification_fail() {
    let fib = Box::new(super::MulFib2Example::new(build_proof_options(false)));
    crate::tests::test_basic_proof_verification_fail(fib, 16);
}
