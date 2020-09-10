use crate::{field, utils::remove_leading_zeros};

#[test]
fn eval() {
    let x: u128 = 11269864713250585702;
    let poly: [u128; 4] = [
        384863712573444386,
        7682273369345308472,
        13294661765012277990,
        16234810094004944758,
    ];

    assert_eq!(0, super::eval(&[], x));

    // constant
    assert_eq!(poly[0], super::eval(&poly[..1], x));

    // degree 1
    assert_eq!(
        field::add(poly[0], field::mul(poly[1], x)),
        super::eval(&poly[..2], x)
    );

    // degree 2
    let x2 = field::exp(x, 2);
    assert_eq!(
        field::add(
            poly[0],
            field::add(field::mul(poly[1], x), field::mul(poly[2], x2))
        ),
        super::eval(&poly[..3], x)
    );

    // degree 3
    let x3 = field::exp(x, 3);
    assert_eq!(
        field::add(
            poly[0],
            field::add(
                field::mul(poly[1], x),
                field::add(field::mul(poly[2], x2), field::mul(poly[3], x3))
            )
        ),
        super::eval(&poly, x)
    );
}

#[test]
fn add() {
    let poly1: [u128; 3] = [
        384863712573444386,
        7682273369345308472,
        13294661765012277990,
    ];
    let poly2: [u128; 3] = [
        9918505539874556741,
        16401861429499852246,
        12181445947541805654,
    ];

    // same degree
    let pr = vec![
        field::add(poly1[0], poly2[0]),
        field::add(poly1[1], poly2[1]),
        field::add(poly1[2], poly2[2]),
    ];
    assert_eq!(pr, super::add(&poly1, &poly2));

    // poly1 is lower degree
    let pr = vec![
        field::add(poly1[0], poly2[0]),
        field::add(poly1[1], poly2[1]),
        poly2[2],
    ];
    assert_eq!(pr, super::add(&poly1[..2], &poly2));

    // poly2 is lower degree
    let pr = vec![
        field::add(poly1[0], poly2[0]),
        field::add(poly1[1], poly2[1]),
        poly1[2],
    ];
    assert_eq!(pr, super::add(&poly1, &poly2[..2]));
}

#[test]
fn sub() {
    let poly1: [u128; 3] = [
        384863712573444386,
        7682273369345308472,
        13294661765012277990,
    ];
    let poly2: [u128; 3] = [
        9918505539874556741,
        16401861429499852246,
        12181445947541805654,
    ];

    // same degree
    let pr = vec![
        field::sub(poly1[0], poly2[0]),
        field::sub(poly1[1], poly2[1]),
        field::sub(poly1[2], poly2[2]),
    ];
    assert_eq!(pr, super::sub(&poly1, &poly2));

    // poly1 is lower degree
    let pr = vec![
        field::sub(poly1[0], poly2[0]),
        field::sub(poly1[1], poly2[1]),
        field::sub(0, poly2[2]),
    ];
    assert_eq!(pr, super::sub(&poly1[..2], &poly2));

    // poly2 is lower degree
    let pr = vec![
        field::sub(poly1[0], poly2[0]),
        field::sub(poly1[1], poly2[1]),
        field::sub(poly1[2], 0),
    ];
    assert_eq!(pr, super::sub(&poly1, &poly2[..2]));
}

#[test]
fn mul() {
    let poly1: [u128; 3] = [
        384863712573444386,
        7682273369345308472,
        13294661765012277990,
    ];
    let poly2: [u128; 3] = [
        9918505539874556741,
        16401861429499852246,
        12181445947541805654,
    ];

    // same degree
    let pr = vec![
        field::mul(poly1[0], poly2[0]),
        field::add(
            field::mul(poly1[0], poly2[1]),
            field::mul(poly2[0], poly1[1]),
        ),
        field::add(
            field::mul(poly1[1], poly2[1]),
            field::add(
                field::mul(poly1[2], poly2[0]),
                field::mul(poly2[2], poly1[0]),
            ),
        ),
        field::add(
            field::mul(poly1[2], poly2[1]),
            field::mul(poly2[2], poly1[1]),
        ),
        field::mul(poly1[2], poly2[2]),
    ];
    assert_eq!(pr, super::mul(&poly1, &poly2));

    // poly1 is lower degree
    let pr = vec![
        field::mul(poly1[0], poly2[0]),
        field::add(
            field::mul(poly1[0], poly2[1]),
            field::mul(poly2[0], poly1[1]),
        ),
        field::add(
            field::mul(poly1[0], poly2[2]),
            field::mul(poly2[1], poly1[1]),
        ),
        field::mul(poly1[1], poly2[2]),
    ];
    assert_eq!(pr, super::mul(&poly1[..2], &poly2));

    // poly2 is lower degree
    let pr = vec![
        field::mul(poly1[0], poly2[0]),
        field::add(
            field::mul(poly1[0], poly2[1]),
            field::mul(poly2[0], poly1[1]),
        ),
        field::add(
            field::mul(poly1[2], poly2[0]),
            field::mul(poly2[1], poly1[1]),
        ),
        field::mul(poly1[2], poly2[1]),
    ];
    assert_eq!(pr, super::mul(&poly1, &poly2[..2]));
}

#[test]
fn mul_by_const() {
    let poly = [
        384863712573444386,
        7682273369345308472,
        13294661765012277990,
    ];
    let c: u128 = 11269864713250585702;
    let pr = vec![
        field::mul(poly[0], c),
        field::mul(poly[1], c),
        field::mul(poly[2], c),
    ];
    assert_eq!(pr, super::mul_by_const(&poly, c));
}

#[test]
fn div() {
    let poly1: Vec<u128> = vec![
        384863712573444386,
        7682273369345308472,
        13294661765012277990,
    ];
    let poly2: Vec<u128> = vec![
        9918505539874556741,
        16401861429499852246,
        12181445947541805654,
    ];

    // divide degree 4 by degree 2
    let poly3 = super::mul(&poly1, &poly2);
    assert_eq!(poly1, super::div(&poly3, &poly2));

    // divide degree 3 by degree 2
    let poly3 = super::mul(&poly1[..2], &poly2);
    assert_eq!(poly1[..2].to_vec(), super::div(&poly3, &poly2));

    // divide degree 3 by degree 3
    let poly3 = super::mul_by_const(&poly1, 11269864713250585702);
    assert_eq!(vec![11269864713250585702], super::div(&poly3, &poly1));
}

#[test]
fn syn_div() {
    let poly = super::mul(&[2, 1], &[3, 1]);

    let result = super::syn_div(&poly, field::neg(3));
    let expected = super::div(&poly, &[3, 1]);

    assert_eq!(expected, remove_leading_zeros(&result));
}

#[test]
fn syn_div_expanded_in_place() {
    let ys = vec![0, 1, 2, 3, 0, 5, 6, 7, 0, 9, 10, 11, 12, 13, 14, 15];

    // build the domain
    let root = field::get_root_of_unity(ys.len());
    let domain = field::get_power_series(root, ys.len());

    // build the polynomial
    let poly = super::interpolate(&domain, &ys);

    // build the divisor polynomial
    let z_poly = vec![field::neg(field::ONE), 0, 0, 0, 1];
    let z_degree = z_poly.len() - 1;
    let z_poly = super::div(&z_poly, &[field::neg(domain[12]), 1]);

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
    assert_eq!(0, super::degree_of(&[1]));
    assert_eq!(1, super::degree_of(&[1, 2]));
    assert_eq!(1, super::degree_of(&[1, 2, 0]));
    assert_eq!(2, super::degree_of(&[1, 2, 3]));
    assert_eq!(2, super::degree_of(&[1, 2, 3, 0]));
}
