use std::vec;

use super::*;
use num_bigint::BigUint;

#[test]
fn test_add() {
    // identity
    let r = SmallFieldElement13::rand();
    assert_eq!(r, r + SmallFieldElement13::ZERO);

    // test addition within bounds
    assert_eq!(
        SmallFieldElement13::from(5u8),
        SmallFieldElement13::from(2u8) + SmallFieldElement13::from(3u8)
    );

    // test overflow
    let t = SmallFieldElement13::from(SmallFieldElement13::MODULUS - 1);
    assert_eq!(SmallFieldElement13::ZERO, t + SmallFieldElement13::ONE);
    assert_eq!(SmallFieldElement13::ONE, t + SmallFieldElement13::from(2u8));

    // test random values
    let r1 = SmallFieldElement13::rand();
    let r2 = SmallFieldElement13::rand();

    let expected = (r1.to_big_uint() + r2.to_big_uint()) % BigUint::from(M);
    let expected = SmallFieldElement13::from_big_uint(expected);
    assert_eq!(expected, r1 + r2);
}

#[test]
fn test_sub() {
    // identity
    let r = SmallFieldElement13::rand();
    assert_eq!(r, r - SmallFieldElement13::ZERO);

    // test subtraction within bounds
    assert_eq!(
        SmallFieldElement13::from(2u8),
        SmallFieldElement13::from(5u8) - SmallFieldElement13::from(3u8)
    );

    // test underflow
    let expected = SmallFieldElement13::from(SmallFieldElement13::MODULUS - 2);
    assert_eq!(
        expected,
        SmallFieldElement13::from(3u8) - SmallFieldElement13::from(5u8)
    );
}

#[test]
fn test_mul() {
    // identity
    let r = SmallFieldElement13::rand();
    assert_eq!(SmallFieldElement13::ZERO, r * SmallFieldElement13::ZERO);
    assert_eq!(r, r * SmallFieldElement13::ONE);

    // test multiplication within bounds
    assert_eq!(
        SmallFieldElement13::from(6u8),
        SmallFieldElement13::from(2u8) * SmallFieldElement13::from(3u8)
    );

    // test overflow
    let m = SmallFieldElement13::MODULUS;
    let t = SmallFieldElement13::from(m - 1);
    assert_eq!(SmallFieldElement13::ONE, t * t);
    assert_eq!(
        SmallFieldElement13::from(m - 2),
        t * SmallFieldElement13::from(2u8)
    );
    assert_eq!(
        SmallFieldElement13::from(m - 4),
        t * SmallFieldElement13::from(4u8)
    );

    let t = (m + 1) / 2;
    assert_eq!(
        SmallFieldElement13::ONE,
        SmallFieldElement13::from(t) * SmallFieldElement13::from(2u8)
    );

    // test random values
    let v1 = SmallFieldElement13::prng_vector(build_seed(), 1000);
    let v2 = SmallFieldElement13::prng_vector(build_seed(), 1000);
    for i in 0..v1.len() {
        let r1 = v1[i];
        let r2 = v2[i];

        let expected = (r1.to_big_uint() * r2.to_big_uint()) % BigUint::from(M);
        let expected = SmallFieldElement13::from_big_uint(expected);

        if expected != r1 * r2 {
            println!("failed for: {} * {}", r1, r2);
            assert_eq!(expected, r1 * r2);
        }
    }
}

#[test]
fn test_inv() {
    // identity
    assert_eq!(
        SmallFieldElement13::ONE,
        SmallFieldElement13::inv(SmallFieldElement13::ONE)
    );
    assert_eq!(
        SmallFieldElement13::ZERO,
        SmallFieldElement13::inv(SmallFieldElement13::ZERO)
    );

    // test random values
    let x = SmallFieldElement13::prng_vector(build_seed(), 5);
    for i in 0..x.len() {
        let y = SmallFieldElement13::inv(x[i]);
        println!("x = {}; y = {}", x[i], y);
        assert!(x[i] == SmallFieldElement13::ZERO || SmallFieldElement13::ONE == x[i] * y);
    }
}

#[test]
fn test_get_root_of_unity() {
    let root_12 = SmallFieldElement13::get_root_of_unity(12);
    assert_eq!(SmallFieldElement13::from(2u32), root_12);

    let powers: Vec<u32> = vec![2, 4, 8, 3, 6, 12, 11, 9, 5, 10, 7, 1];
    for i in 1..13 {
        assert_eq!(
            SmallFieldElement13::from(powers[i - 1]),
            SmallFieldElement13::exp(root_12, i.try_into().unwrap())
        );
    }

    let root_2 = SmallFieldElement13::get_root_of_unity(2);

    let expected = SmallFieldElement13::exp(root_12, 6);
    assert_eq!(expected, root_2);
    assert_eq!(
        SmallFieldElement13::ONE,
        SmallFieldElement13::exp(root_2, 2)
    );
}

#[test]
fn test_array_as_bytes() {
    let source: &[SmallFieldElement13; 4] = &[
        SmallFieldElement13::new(1),
        SmallFieldElement13::new(2),
        SmallFieldElement13::new(3),
        SmallFieldElement13::new(4),
    ];

    // should convert correctly
    let expected: Vec<u8> = vec![
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];
    assert_eq!(expected, source.as_bytes());
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_seed() -> [u8; 32] {
    let mut result = [0; 32];
    let random_elt = SmallFieldElement13::rand();
    let seed = random_elt.as_bytes();
    result[..16].copy_from_slice(&seed);
    result
}

impl SmallFieldElement13 {
    pub fn to_big_uint(&self) -> BigUint {
        BigUint::from_bytes_le(&self.as_bytes())
    }

    pub fn from_big_uint(value: BigUint) -> Self {
        let bytes = value.to_bytes_le();
        let mut buffer = [0u8; 16];
        buffer[0..bytes.len()].copy_from_slice(&bytes);
        SmallFieldElement13::try_from(buffer).unwrap()
    }
}
