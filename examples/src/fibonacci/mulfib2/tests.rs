use super::*;

#[cfg(test)]
mod multifib2_tests {
    
    
    #[test]
    fn multifib2_test_basic_proof_verification() {
        let mut fib = super::get_example();
        crate::tests::test_basic_proof_verification(fib, Some(16), Some(8), Some(32), Some(0));
    }

    #[test]
    fn multifib2_test_basic_proof_verification_fail() {
        let mut fib = super::get_example();
        crate::tests::test_basic_proof_verification_fail(fib, Some(16), Some(8), Some(32), Some(0));
    }

}