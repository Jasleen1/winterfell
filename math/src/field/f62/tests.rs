use super::{BaseElement, FieldElement, StarkField};
use num_bigint::BigUint;
use proptest::prelude::*;

// MANUAL TESTS
// ================================================================================================

#[test]
fn add() {
    // identity
    let r = BaseElement::rand();
    assert_eq!(r, r + BaseElement::ZERO);

    // test addition within bounds
    assert_eq!(
        BaseElement::from(5u8),
        BaseElement::from(2u8) + BaseElement::from(3u8)
    );

    // test overflow
    let t = BaseElement::from(BaseElement::MODULUS - 1);
    assert_eq!(BaseElement::ZERO, t + BaseElement::ONE);
    assert_eq!(BaseElement::ONE, t + BaseElement::from(2u8));
}

#[test]
fn sub() {
    // identity
    let r = BaseElement::rand();
    assert_eq!(r, r - BaseElement::ZERO);

    // test subtraction within bounds
    assert_eq!(
        BaseElement::from(2u8),
        BaseElement::from(5u8) - BaseElement::from(3u8)
    );

    // test underflow
    let expected = BaseElement::from(BaseElement::MODULUS - 2);
    assert_eq!(expected, BaseElement::from(3u8) - BaseElement::from(5u8));
}

#[test]
fn mul() {
    // identity
    let r = BaseElement::rand();
    assert_eq!(BaseElement::ZERO, r * BaseElement::ZERO);
    assert_eq!(r, r * BaseElement::ONE);

    // test multiplication within bounds
    assert_eq!(
        BaseElement::from(15u8),
        BaseElement::from(5u8) * BaseElement::from(3u8)
    );

    // test overflow
    let m = BaseElement::MODULUS;
    let t = BaseElement::from(m - 1);
    assert_eq!(BaseElement::ONE, t * t);
    assert_eq!(BaseElement::from(m - 2), t * BaseElement::from(2u8));
    assert_eq!(BaseElement::from(m - 4), t * BaseElement::from(4u8));

    let t = (m + 1) / 2;
    assert_eq!(
        BaseElement::ONE,
        BaseElement::from(t) * BaseElement::from(2u8)
    );
}

#[test]
fn exp() {
    let a = BaseElement::ZERO;
    assert_eq!(a.exp(0), BaseElement::ONE);
    assert_eq!(a.exp(1), BaseElement::ZERO);

    let a = BaseElement::ONE;
    assert_eq!(a.exp(0), BaseElement::ONE);
    assert_eq!(a.exp(1), BaseElement::ONE);
    assert_eq!(a.exp(3), BaseElement::ONE);

    let a = BaseElement::rand();
    assert_eq!(a.exp(3), a * a * a);
}

#[test]
fn inv() {
    // identity
    assert_eq!(BaseElement::ONE, BaseElement::inv(BaseElement::ONE));
    assert_eq!(BaseElement::ZERO, BaseElement::inv(BaseElement::ZERO));
}

#[test]
fn element_as_int() {
    let v = u64::MAX;
    let e = BaseElement::new(v);
    assert_eq!(v % super::M, e.as_int());
}

#[test]
fn from_u128() {
    let v = u128::MAX;
    let e = BaseElement::from(v);
    assert_eq!((v % super::M as u128) as u64, e.as_int());
}

// RANDOMIZED TESTS
// ================================================================================================

proptest! {

    #[test]
    fn add_proptest(a in any::<u64>(), b in any::<u64>()) {
        let v1 = BaseElement::from(a);
        let v2 = BaseElement::from(b);
        let result = v1 + v2;

        let expected = (a % super::M + b % super::M) % super::M;
        prop_assert_eq!(expected, result.as_int());
    }

    #[test]
    fn sub_proptest(a in any::<u64>(), b in any::<u64>()) {
        let v1 = BaseElement::from(a);
        let v2 = BaseElement::from(b);
        let result = v1 - v2;

        let a = a % super::M;
        let b = b % super::M;
        let expected = if a < b { super::M - b + a } else { a - b };

        prop_assert_eq!(expected, result.as_int());
    }

    #[test]
    fn mul_proptest(a in any::<u64>(), b in any::<u64>()) {
        let v1 = BaseElement::from(a);
        let v2 = BaseElement::from(b);
        let result = v1 * v2;

        let expected = (((a as u128) * (b as u128)) % super::M as u128) as u64;
        prop_assert_eq!(expected, result.as_int());
    }

    #[test]
    fn exp_proptest(a in any::<u64>(), b in any::<u64>()) {
        let result = BaseElement::from(a).exp(b);

        let b = BigUint::from(b);
        let m = BigUint::from(super::M);
        let expected = BigUint::from(a).modpow(&b, &m).to_u64_digits()[0];
        prop_assert_eq!(expected, result.as_int());
    }

    #[test]
    fn inv_proptest(a in any::<u64>()) {
        let a = BaseElement::from(a);
        let b = a.inv();

        let expected = if a == BaseElement::ZERO { BaseElement::ZERO } else { BaseElement::ONE };
        prop_assert_eq!(expected, a * b);
    }

    #[test]
    fn element_as_int_proptest(a in any::<u64>()) {
        let e = BaseElement::new(a);
        prop_assert_eq!(a % super::M, e.as_int());
    }

    #[test]
    fn from_u128_proptest(v in any::<u128>()) {
        let e = BaseElement::from(v);
        assert_eq!((v % super::M as u128) as u64, e.as_int());
    }
}
