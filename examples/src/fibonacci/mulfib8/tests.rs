use super::super::utils::build_proof_options;

#[test]
fn mulfib8_test_basic_proof_verification() {
    let fib = Box::new(super::MulFib8Example::new(build_proof_options(false)));
    crate::tests::test_basic_proof_verification(fib, 64);
}

#[test]
fn mulfib8_test_basic_proof_verification_extension() {
    let fib = Box::new(super::MulFib8Example::new(build_proof_options(true)));
    crate::tests::test_basic_proof_verification(fib, 64);
}

#[test]
fn mulfib8_test_basic_proof_verification_fail() {
    let fib = Box::new(super::MulFib8Example::new(build_proof_options(false)));
    crate::tests::test_basic_proof_verification_fail(fib, 64);
}
