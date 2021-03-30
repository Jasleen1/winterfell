use super::*;
use num_bigint::BigUint;

#[test]
fn test_add() {
    // identity
    let r = SmallFieldElement7::rand();
    assert_eq!(r, r + SmallFieldElement7::ZERO);

    // test addition within bounds
    assert_eq!(
        SmallFieldElement7::from(5u8),
        SmallFieldElement7::from(2u8) + SmallFieldElement7::from(3u8)
    );

    // test overflow
    let t = SmallFieldElement7::from(SmallFieldElement7::MODULUS - 1);
    assert_eq!(SmallFieldElement7::ZERO, t + SmallFieldElement7::ONE);
    assert_eq!(SmallFieldElement7::ONE, t + SmallFieldElement7::from(2u8));

    // test random values
    let r1 = SmallFieldElement7::rand();
    let r2 = SmallFieldElement7::rand();

    let expected = (r1.to_big_uint() + r2.to_big_uint()) % BigUint::from(M);
    let expected = SmallFieldElement7::from_big_uint(expected);
    assert_eq!(expected, r1 + r2);
}

#[test]
fn test_sub() {
    // identity
    let r = SmallFieldElement7::rand();
    assert_eq!(r, r - SmallFieldElement7::ZERO);

    // test subtraction within bounds
    assert_eq!(
        SmallFieldElement7::from(2u8),
        SmallFieldElement7::from(5u8) - SmallFieldElement7::from(3u8)
    );

    // test underflow
    let expected = SmallFieldElement7::from(SmallFieldElement7::MODULUS - 2);
    assert_eq!(expected, SmallFieldElement7::from(3u8) - SmallFieldElement7::from(5u8));
}

#[test]
fn test_mul() {
    // identity
    let r = SmallFieldElement7::rand();
    assert_eq!(SmallFieldElement7::ZERO, r * SmallFieldElement7::ZERO);
    assert_eq!(r, r * SmallFieldElement7::ONE);

    // test multiplication within bounds
    assert_eq!(
        SmallFieldElement7::from(6u8),
        SmallFieldElement7::from(2u8) * SmallFieldElement7::from(3u8)
    );

    // test overflow
    let m = SmallFieldElement7::MODULUS;
    let t = SmallFieldElement7::from(m - 1);
    assert_eq!(SmallFieldElement7::ONE, t * t);
    assert_eq!(SmallFieldElement7::from(m - 2), t * SmallFieldElement7::from(2u8));
    assert_eq!(SmallFieldElement7::from(m - 4), t * SmallFieldElement7::from(4u8));

    let t = (m + 1) / 2;
    assert_eq!(
        SmallFieldElement7::ONE,
        SmallFieldElement7::from(t) * SmallFieldElement7::from(2u8)
    );

    // test random values
    let v1 = SmallFieldElement7::prng_vector(build_seed(), 1000);
    let v2 = SmallFieldElement7::prng_vector(build_seed(), 1000);
    for i in 0..v1.len() {
        let r1 = v1[i];
        let r2 = v2[i];

        let expected = (r1.to_big_uint() * r2.to_big_uint()) % BigUint::from(M);
        let expected = SmallFieldElement7::from_big_uint(expected);

        if expected != r1 * r2 {
            println!("failed for: {} * {}", r1, r2);
            assert_eq!(expected, r1 * r2);
        }
    }
}

#[test]
fn test_inv() {
    // identity
    assert_eq!(SmallFieldElement7::ONE, SmallFieldElement7::inv(SmallFieldElement7::ONE));
    assert_eq!(SmallFieldElement7::ZERO, SmallFieldElement7::inv(SmallFieldElement7::ZERO));

    // test random values
    let x = SmallFieldElement7::prng_vector(build_seed(), 5);
    for i in 0..x.len() {
        let y = SmallFieldElement7::inv(x[i]);
        println!("x = {}; y = {}", x[i], y);
        assert!(x[i] == SmallFieldElement7::ZERO || SmallFieldElement7::ONE == x[i] * y);
    }
}

#[test]
fn test_get_root_of_unity() {
    let root_6 = SmallFieldElement7::get_root_of_unity(6);
    assert_eq!(
        SmallFieldElement7::from(3u32),
        root_6
    );
    println!("root_6 {}", root_6);
    for i in 1..6 {
        println!("root_6 pow {} {}", i, SmallFieldElement7::exp(root_6, i));
    }
    assert_eq!(
        SmallFieldElement7::ONE,
        SmallFieldElement7::exp(root_6, 6)
    );

    let root_2 = SmallFieldElement7::get_root_of_unity(2);
    println!("mul_test {}", root_2 * root_2);
    for i in 1..6 {
        println!("root_2 pow {} {}", i, SmallFieldElement7::exp(root_2, i));
    }
    
    let expected = SmallFieldElement7::exp(root_6, 3);
    assert_eq!(expected, root_2);
    assert_eq!(
        SmallFieldElement7::ONE,
        SmallFieldElement7::exp(root_2, 2)
    );
}



#[test]
fn test_array_as_bytes() {
    let source: &[SmallFieldElement7; 4] = &[
        SmallFieldElement7::new(1),
        SmallFieldElement7::new(2),
        SmallFieldElement7::new(3),
        SmallFieldElement7::new(4),
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
    let seed = SmallFieldElement7::rand().to_bytes();
    result[..16].copy_from_slice(&seed);
    result
}

impl SmallFieldElement7 {
    pub fn to_big_uint(&self) -> BigUint {
        BigUint::from_bytes_le(&self.to_bytes())
    }

    pub fn from_big_uint(value: BigUint) -> Self {
        let bytes = value.to_bytes_le();
        let mut buffer = [0u8; 16];
        buffer[0..bytes.len()].copy_from_slice(&bytes);
        SmallFieldElement7::try_from(buffer).unwrap()
    }
}
