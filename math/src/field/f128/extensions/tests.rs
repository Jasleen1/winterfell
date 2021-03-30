use super::{QuadElement, FieldElement, SerializationError, AsBytes};

// SERIALIZATION / DESERIALIZATION
// ================================================================================================

#[test]
fn test_array_as_bytes() {
    let source: &[QuadElement; 2] = &[
        QuadElement::new(1, 2),
        QuadElement::new(3, 4),
    ];

    // should convert correctly
    let expected: Vec<u8> = vec![
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];
    assert_eq!(expected, source.as_bytes());
}

#[test]
fn test_elements_as_bytes() {
    let source = vec![
        QuadElement::new(1, 2),
        QuadElement::new(3, 4),
    ];

    let expected: Vec<u8> = vec![
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];

    assert_eq!(expected, QuadElement::elements_as_bytes(&source));
}

#[test]
fn test_bytes_as_elements() {

    let bytes: Vec<u8> = vec![
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 5
    ];

    let expected = vec![
        QuadElement::new(1, 2),
        QuadElement::new(3, 4),
    ];

    let result = unsafe { QuadElement::bytes_as_elements(&bytes[..64]) };
    assert!(result.is_ok());
    assert_eq!(expected, result.unwrap());

    let result = unsafe { QuadElement::bytes_as_elements(&bytes) };
    assert_eq!(result, Err(SerializationError::NotEnoughBytesForWholeElements(65)));

    let result = unsafe { QuadElement::bytes_as_elements(&bytes[1..]) };
    assert_eq!(result, Err(SerializationError::InvalidMemoryAlignment));
}
