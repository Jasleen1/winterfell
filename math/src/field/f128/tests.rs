use super::*;
use num_bigint::BigUint;

#[test]
fn test_add() {
    // identity
    let r = FieldElement::rand();
    assert_eq!(r, r + FieldElement::ZERO);

    // test addition within bounds
    assert_eq!(
        FieldElement::from(5u8),
        FieldElement::from(2u8) + FieldElement::from(3u8)
    );

    // test overflow
    let t = FieldElement::from(FieldElement::MODULUS - 1);
    assert_eq!(FieldElement::ZERO, t + FieldElement::ONE);
    assert_eq!(FieldElement::ONE, t + FieldElement::from(2u8));

    // test random values
    let r1 = FieldElement::rand();
    let r2 = FieldElement::rand();

    let expected = (r1.to_big_uint() + r2.to_big_uint()) % BigUint::from(M);
    let expected = FieldElement::from_big_uint(expected);
    assert_eq!(expected, r1 + r2);
}

#[test]
fn test_sub() {
    // identity
    let r = FieldElement::rand();
    assert_eq!(r, r - FieldElement::ZERO);

    // test subtraction within bounds
    assert_eq!(
        FieldElement::from(2u8),
        FieldElement::from(5u8) - FieldElement::from(3u8)
    );

    // test underflow
    let expected = FieldElement::from(FieldElement::MODULUS - 2);
    assert_eq!(expected, FieldElement::from(3u8) - FieldElement::from(5u8));
}

#[test]
fn test_mul() {
    // identity
    let r = FieldElement::rand();
    assert_eq!(FieldElement::ZERO, r * FieldElement::ZERO);
    assert_eq!(r, r * FieldElement::ONE);

    // test multiplication within bounds
    assert_eq!(
        FieldElement::from(15u8),
        FieldElement::from(5u8) * FieldElement::from(3u8)
    );

    // test overflow
    let m = FieldElement::MODULUS;
    let t = FieldElement::from(m - 1);
    assert_eq!(FieldElement::ONE, t * t);
    assert_eq!(FieldElement::from(m - 2), t * FieldElement::from(2u8));
    assert_eq!(FieldElement::from(m - 4), t * FieldElement::from(4u8));

    let t = (m + 1) / 2;
    assert_eq!(
        FieldElement::ONE,
        FieldElement::from(t) * FieldElement::from(2u8)
    );

    // test random values
    let v1 = FieldElement::prng_vector(build_seed(), 1000);
    let v2 = FieldElement::prng_vector(build_seed(), 1000);
    for i in 0..v1.len() {
        let r1 = v1[i];
        let r2 = v2[i];

        let expected = (r1.to_big_uint() * r2.to_big_uint()) % BigUint::from(M);
        let expected = FieldElement::from_big_uint(expected);

        if expected != r1 * r2 {
            println!("failed for: {} * {}", r1, r2);
            assert_eq!(expected, r1 * r2);
        }
    }
}

#[test]
fn test_inv() {
    // identity
    assert_eq!(FieldElement::ONE, FieldElement::inv(FieldElement::ONE));
    assert_eq!(FieldElement::ZERO, FieldElement::inv(FieldElement::ZERO));

    // test random values
    let x = FieldElement::prng_vector(build_seed(), 1000);
    for i in 0..x.len() {
        let y = FieldElement::inv(x[i]);
        assert_eq!(FieldElement::ONE, x[i] * y);
    }
}

#[test]
fn test_get_root_of_unity() {
    let root_40 = FieldElement::get_root_of_unity(40);
    assert_eq!(
        FieldElement::from(23953097886125630542083529559205016746u128),
        root_40
    );
    assert_eq!(
        FieldElement::ONE,
        FieldElement::exp(root_40, u128::pow(2, 40))
    );

    let root_39 = FieldElement::get_root_of_unity(39);
    let expected = FieldElement::exp(root_40, 2);
    assert_eq!(expected, root_39);
    assert_eq!(
        FieldElement::ONE,
        FieldElement::exp(root_39, u128::pow(2, 39))
    );
}

#[test]
fn test_g_is_2_exp_40_root() {
    assert_eq!(exp(G, 1u128 << 40), 1u128)
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_seed() -> [u8; 32] {
    let mut result = [0; 32];
    let seed = FieldElement::rand().to_bytes();
    result[..16].copy_from_slice(&seed);
    result
}

impl FieldElement {
    pub fn to_big_uint(&self) -> BigUint {
        BigUint::from_bytes_le(&self.to_bytes())
    }

    pub fn from_big_uint(value: BigUint) -> Self {
        let bytes = value.to_bytes_le();
        let mut buffer = [0u8; 16];
        buffer[0..bytes.len()].copy_from_slice(&bytes);
        FieldElement::try_from(buffer).unwrap()
    }
}
