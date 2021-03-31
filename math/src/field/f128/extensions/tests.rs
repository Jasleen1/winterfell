use super::super::StarkField;
use super::{AsBytes, FieldElement, QuadElement, SerializationError};

// BASIC ALGEBRA
// ================================================================================================

#[test]
fn add() {
    // identity
    let r = QuadElement::rand();
    assert_eq!(r, r + QuadElement::ZERO);

    // test random values
    let r1 = QuadElement::rand();
    let r2 = QuadElement::rand();

    let expected = QuadElement::new((r1.0 + r2.0).as_int(), (r1.1 + r2.1).as_int());
    assert_eq!(expected, r1 + r2);
}

#[test]
fn sub() {
    // identity
    let r = QuadElement::rand();
    assert_eq!(r, r - QuadElement::ZERO);

    // test random values
    let r1 = QuadElement::rand();
    let r2 = QuadElement::rand();

    let expected = QuadElement::new((r1.0 - r2.0).as_int(), (r1.1 - r2.1).as_int());
    assert_eq!(expected, r1 - r2);
}

#[test]
fn mul() {
    // identity
    let r = QuadElement::rand();
    assert_eq!(QuadElement::ZERO, r * QuadElement::ZERO);
    assert_eq!(r, r * QuadElement::ONE);

    // test random values
    let r1 = QuadElement::rand();
    let r2 = QuadElement::rand();

    let expected = QuadElement::new(
        (r1.0 * r2.0 + r1.1 * r2.1).as_int(),
        ((r1.0 + r1.1) * (r2.0 + r2.1) - r1.0 * r2.0).as_int(),
    );
    assert_eq!(expected, r1 * r2);
}

#[test]
fn inv() {
    // identity
    assert_eq!(QuadElement::ONE, QuadElement::inv(QuadElement::ONE));
    assert_eq!(QuadElement::ZERO, QuadElement::inv(QuadElement::ZERO));

    // test random values
    let x = QuadElement::prng_vector(build_seed(), 1000);
    for i in 0..x.len() {
        let y = QuadElement::inv(x[i]);
        assert_eq!(QuadElement::ONE, x[i] * y);
    }
}

#[test]
fn conjugate() {
    let a = QuadElement::rand();
    let b = a.conjugate();
    let expected = QuadElement::new((a.0 + a.1).as_int(), (-a.1).as_int());
    assert_eq!(expected, b);
}

// INITIALIZATION
// ================================================================================================

#[test]
fn zeroed_vector() {
    let result = QuadElement::zeroed_vector(4);
    assert_eq!(4, result.len());
    for element in result.into_iter() {
        assert_eq!(QuadElement::ZERO, element);
    }
}

#[test]
fn prng_vector() {
    let a = QuadElement::prng_vector([0; 32], 4);
    assert_eq!(4, a.len());

    let b = QuadElement::prng_vector([0; 32], 8);
    assert_eq!(8, b.len());

    for (&a, &b) in a.iter().zip(b.iter()) {
        assert_eq!(a, b);
    }

    let c = QuadElement::prng_vector([1; 32], 4);
    for (&a, &c) in a.iter().zip(c.iter()) {
        assert_ne!(a, c);
    }
}

// SERIALIZATION / DESERIALIZATION
// ================================================================================================

#[test]
fn elements_into_bytes() {
    let source = vec![QuadElement::new(1, 2), QuadElement::new(3, 4)];

    let expected: Vec<u8> = vec![
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];

    assert_eq!(expected, QuadElement::elements_into_bytes(source));
}

#[test]
fn array_as_bytes() {
    let source: &[QuadElement; 2] = &[QuadElement::new(1, 2), QuadElement::new(3, 4)];

    // should convert correctly
    let expected: Vec<u8> = vec![
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];
    assert_eq!(expected, source.as_bytes());
}

#[test]
fn elements_as_bytes() {
    let source = vec![QuadElement::new(1, 2), QuadElement::new(3, 4)];

    let expected: Vec<u8> = vec![
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];

    assert_eq!(expected, QuadElement::elements_as_bytes(&source));
}

#[test]
fn bytes_as_elements() {
    let bytes: Vec<u8> = vec![
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 5,
    ];

    let expected = vec![QuadElement::new(1, 2), QuadElement::new(3, 4)];

    let result = unsafe { QuadElement::bytes_as_elements(&bytes[..64]) };
    assert!(result.is_ok());
    assert_eq!(expected, result.unwrap());

    let result = unsafe { QuadElement::bytes_as_elements(&bytes) };
    assert_eq!(
        result,
        Err(SerializationError::NotEnoughBytesForWholeElements(65))
    );

    let result = unsafe { QuadElement::bytes_as_elements(&bytes[1..]) };
    assert_eq!(result, Err(SerializationError::InvalidMemoryAlignment));
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_seed() -> [u8; 32] {
    let mut result = [0; 32];
    let seed = QuadElement::rand().as_bytes().to_vec();
    result.copy_from_slice(&seed);
    result
}
