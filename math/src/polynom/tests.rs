use crate::{
    field::{FieldElement, StarkField},
    utils::remove_leading_zeros,
};

#[test]
fn eval() {
    let x = FieldElement::from(11269864713250585702u128);
    let poly: [FieldElement; 4] = [
        FieldElement::from(384863712573444386u128),
        FieldElement::from(7682273369345308472u128),
        FieldElement::from(13294661765012277990u128),
        FieldElement::from(16234810094004944758u128),
    ];

    assert_eq!(FieldElement::ZERO, super::eval(&[], x));

    // constant
    assert_eq!(poly[0], super::eval(&poly[..1], x));

    // degree 1
    assert_eq!(poly[0] + poly[1] * x, super::eval(&poly[..2], x));

    // degree 2
    let x2 = FieldElement::exp(x, 2);
    assert_eq!(
        poly[0] + poly[1] * x + poly[2] * x2,
        super::eval(&poly[..3], x)
    );

    // degree 3
    let x3 = FieldElement::exp(x, 3);
    assert_eq!(
        poly[0] + poly[1] * x + poly[2] * x2 + poly[3] * x3,
        super::eval(&poly, x)
    );
}

#[test]
fn add() {
    let poly1: [FieldElement; 3] = [
        FieldElement::from(384863712573444386u128),
        FieldElement::from(7682273369345308472u128),
        FieldElement::from(13294661765012277990u128),
    ];
    let poly2: [FieldElement; 3] = [
        FieldElement::from(9918505539874556741u128),
        FieldElement::from(16401861429499852246u128),
        FieldElement::from(12181445947541805654u128),
    ];

    // same degree
    let pr = vec![
        poly1[0] + poly2[0],
        poly1[1] + poly2[1],
        poly1[2] + poly2[2],
    ];
    assert_eq!(pr, super::add(&poly1, &poly2));

    // poly1 is lower degree
    let pr = vec![poly1[0] + poly2[0], poly1[1] + poly2[1], poly2[2]];
    assert_eq!(pr, super::add(&poly1[..2], &poly2));

    // poly2 is lower degree
    let pr = vec![poly1[0] + poly2[0], poly1[1] + poly2[1], poly1[2]];
    assert_eq!(pr, super::add(&poly1, &poly2[..2]));
}

#[test]
fn sub() {
    let poly1: [FieldElement; 3] = [
        FieldElement::from(384863712573444386u128),
        FieldElement::from(7682273369345308472u128),
        FieldElement::from(13294661765012277990u128),
    ];
    let poly2: [FieldElement; 3] = [
        FieldElement::from(9918505539874556741u128),
        FieldElement::from(16401861429499852246u128),
        FieldElement::from(12181445947541805654u128),
    ];

    // same degree
    let pr = vec![
        poly1[0] - poly2[0],
        poly1[1] - poly2[1],
        poly1[2] - poly2[2],
    ];
    assert_eq!(pr, super::sub(&poly1, &poly2));

    // poly1 is lower degree
    let pr = vec![poly1[0] - poly2[0], poly1[1] - poly2[1], -poly2[2]];
    assert_eq!(pr, super::sub(&poly1[..2], &poly2));

    // poly2 is lower degree
    let pr = vec![poly1[0] - poly2[0], poly1[1] - poly2[1], poly1[2]];
    assert_eq!(pr, super::sub(&poly1, &poly2[..2]));
}

#[test]
fn mul() {
    let poly1: [FieldElement; 3] = [
        FieldElement::from(384863712573444386u128),
        FieldElement::from(7682273369345308472u128),
        FieldElement::from(13294661765012277990u128),
    ];
    let poly2: [FieldElement; 3] = [
        FieldElement::from(9918505539874556741u128),
        FieldElement::from(16401861429499852246u128),
        FieldElement::from(12181445947541805654u128),
    ];

    // same degree
    let pr = vec![
        poly1[0] * poly2[0],
        poly1[0] * poly2[1] + poly2[0] * poly1[1],
        poly1[1] * poly2[1] + poly1[2] * poly2[0] + poly2[2] * poly1[0],
        poly1[2] * poly2[1] + poly2[2] * poly1[1],
        poly1[2] * poly2[2],
    ];
    assert_eq!(pr, super::mul(&poly1, &poly2));

    // poly1 is lower degree
    let pr = vec![
        poly1[0] * poly2[0],
        poly1[0] * poly2[1] + poly2[0] * poly1[1],
        poly1[0] * poly2[2] + poly2[1] * poly1[1],
        poly1[1] * poly2[2],
    ];
    assert_eq!(pr, super::mul(&poly1[..2], &poly2));

    // poly2 is lower degree
    let pr = vec![
        poly1[0] * poly2[0],
        poly1[0] * poly2[1] + poly2[0] * poly1[1],
        poly1[2] * poly2[0] + poly2[1] * poly1[1],
        poly1[2] * poly2[1],
    ];
    assert_eq!(pr, super::mul(&poly1, &poly2[..2]));
}

#[test]
fn mul_by_const() {
    let poly = [
        FieldElement::from(384863712573444386u128),
        FieldElement::from(7682273369345308472u128),
        FieldElement::from(13294661765012277990u128),
    ];
    let c = FieldElement::from(11269864713250585702u128);
    let pr = vec![poly[0] * c, poly[1] * c, poly[2] * c];
    assert_eq!(pr, super::mul_by_const(&poly, c));
}

#[test]
fn div() {
    let poly1 = vec![
        FieldElement::from(384863712573444386u128),
        FieldElement::from(7682273369345308472u128),
        FieldElement::from(13294661765012277990u128),
    ];
    let poly2 = vec![
        FieldElement::from(9918505539874556741u128),
        FieldElement::from(16401861429499852246u128),
        FieldElement::from(12181445947541805654u128),
    ];

    // divide degree 4 by degree 2
    let poly3 = super::mul(&poly1, &poly2);
    assert_eq!(poly1, super::div(&poly3, &poly2));

    // divide degree 3 by degree 2
    let poly3 = super::mul(&poly1[..2], &poly2);
    assert_eq!(poly1[..2].to_vec(), super::div(&poly3, &poly2));

    // divide degree 3 by degree 3
    let poly3 = super::mul_by_const(&poly1, FieldElement::from(11269864713250585702u128));
    assert_eq!(
        vec![FieldElement::from(11269864713250585702u128)],
        super::div(&poly3, &poly1)
    );
}

#[test]
fn syn_div() {
    let poly = super::mul(
        &[FieldElement::from(2u8), FieldElement::ONE],
        &[FieldElement::from(3u8), FieldElement::ONE],
    );

    let result = super::syn_div(&poly, -FieldElement::from(3u8));
    let expected = super::div(&poly, &[FieldElement::from(3u8), FieldElement::ONE]);

    assert_eq!(expected, remove_leading_zeros(&result));
}

#[test]
fn syn_div_expanded_in_place() {
    let ys: Vec<FieldElement> = vec![0u8, 1, 2, 3, 0, 5, 6, 7, 0, 9, 10, 11, 12, 13, 14, 15]
        .into_iter()
        .map(FieldElement::from)
        .collect();

    // build the domain
    let root = FieldElement::get_root_of_unity(ys.len().trailing_zeros());
    let domain = FieldElement::get_power_series(root, ys.len());

    // build the polynomial
    let poly = super::interpolate(&domain, &ys, false);

    // build the divisor polynomial
    let z_poly = vec![
        -FieldElement::ONE,
        FieldElement::ZERO,
        FieldElement::ZERO,
        FieldElement::ZERO,
        FieldElement::ONE,
    ];
    let z_degree = z_poly.len() - 1;
    let z_poly = super::div(&z_poly, &[-domain[12], FieldElement::ONE]);

    // compute the result
    let mut result = poly.clone();
    super::syn_div_expanded_in_place(&mut result, z_degree, &[domain[12]]);

    let expected = super::div(&poly, &z_poly);

    assert_eq!(expected, remove_leading_zeros(&result));
    assert_eq!(poly, remove_leading_zeros(&super::mul(&expected, &z_poly)));
}

#[test]
fn degree_of() {
    assert_eq!(0, super::degree_of(&[]));
    assert_eq!(0, super::degree_of(&[FieldElement::ONE]));
    assert_eq!(
        1,
        super::degree_of(&[FieldElement::ONE, FieldElement::from(2u8)])
    );
    assert_eq!(
        1,
        super::degree_of(&[
            FieldElement::ONE,
            FieldElement::from(2u8),
            FieldElement::ZERO
        ])
    );
    assert_eq!(
        2,
        super::degree_of(&[
            FieldElement::ONE,
            FieldElement::from(2u8),
            FieldElement::from(3u8)
        ])
    );
    assert_eq!(
        2,
        super::degree_of(&[
            FieldElement::ONE,
            FieldElement::from(2u8),
            FieldElement::from(3u8),
            FieldElement::ZERO
        ])
    );
}
