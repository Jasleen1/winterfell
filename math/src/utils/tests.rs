use crate::field::{BaseElement, FieldElement};

#[test]
fn get_power_series() {
    let n = 1024 * 4; // big enough for concurrent series generation
    let b = BaseElement::from(3u8);

    let mut expected = vec![BaseElement::ZERO; n];
    for (i, value) in expected.iter_mut().enumerate() {
        *value = b.exp((i as u64).into());
    }

    let actual = super::get_power_series(b, n);
    assert_eq!(expected, actual);
}

#[test]
fn get_power_series_with_offset() {
    let n = 1024 * 4; // big enough for concurrent series generation
    let b = BaseElement::from(3u8);
    let s = BaseElement::from(7u8);

    let mut expected = vec![BaseElement::ZERO; n];
    for (i, value) in expected.iter_mut().enumerate() {
        *value = s * b.exp((i as u64).into());
    }

    let actual = super::get_power_series_with_offset(b, s, n);
    assert_eq!(expected, actual);
}

#[test]
fn batch_inversion() {
    let n = 1024 * 4; // big enough for concurrent series generation
    let a = BaseElement::prng_vector([1; 32], n);

    let b = super::batch_inversion(&a);
    for (&a, &b) in a.iter().zip(b.iter()) {
        assert_eq!(a.inv(), b);
    }
}
