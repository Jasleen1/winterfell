use crate::{
    field::{FieldElement, StarkField},
    polynom,
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
    assert_eq!(polynom::eval(&poly, x), super::eval(&poly, x));
}

#[test]
fn interpolate_batch() {
    let r = FieldElement::get_root_of_unity(4);
    let xs = super::to_quartic_vec(FieldElement::get_power_series(r, 16));
    let ys = super::to_quartic_vec(
        vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
            .into_iter()
            .map(FieldElement::from)
            .collect(),
    );

    let mut expected: Vec<[FieldElement; 4]> = vec![];
    for i in 0..xs.len() {
        let mut row = [FieldElement::ZERO; 4];
        row.copy_from_slice(&polynom::interpolate(&xs[i], &ys[i], false));
        expected.push(row);
    }

    assert_eq!(expected, super::interpolate_batch(&xs, &ys));
}

#[test]
fn evaluate_batch() {
    let x = FieldElement::rand();
    let polys: [[FieldElement; 4]; 4] = [
        [
            FieldElement::from(7956382178997078105u128),
            FieldElement::from(6172178935026293282u128),
            FieldElement::from(5971474637801684060u128),
            FieldElement::from(16793452009046991148u128),
        ],
        [
            FieldElement::from(7956382178997078109u128),
            FieldElement::from(15205743380705406848u128),
            FieldElement::from(12475269242634339237u128),
            FieldElement::from(194846859619262948u128),
        ],
        [
            FieldElement::from(7956382178997078113u128),
            FieldElement::from(12274564945409730015u128),
            FieldElement::from(5971474637801684060u128),
            FieldElement::from(1653291871389032149u128),
        ],
        [
            FieldElement::from(7956382178997078117u128),
            FieldElement::from(3241000499730616449u128),
            FieldElement::from(12475269242634339237u128),
            FieldElement::from(18251897020816760349u128),
        ],
    ];

    let expected = vec![
        polynom::eval(&polys[0], x),
        polynom::eval(&polys[1], x),
        polynom::eval(&polys[2], x),
        polynom::eval(&polys[3], x),
    ];
    assert_eq!(expected, super::evaluate_batch(&polys, x));
}

#[test]
fn to_quartic_vec() {
    let vector: Vec<FieldElement> = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        .into_iter()
        .map(FieldElement::from)
        .collect();
    let expected: Vec<[FieldElement; 4]> = vec![
        [
            FieldElement::from(1u8),
            FieldElement::from(2u8),
            FieldElement::from(3u8),
            FieldElement::from(4u8),
        ],
        [
            FieldElement::from(5u8),
            FieldElement::from(6u8),
            FieldElement::from(7u8),
            FieldElement::from(8u8),
        ],
        [
            FieldElement::from(9u8),
            FieldElement::from(10u8),
            FieldElement::from(11u8),
            FieldElement::from(12u8),
        ],
        [
            FieldElement::from(13u8),
            FieldElement::from(14u8),
            FieldElement::from(15u8),
            FieldElement::from(16u8),
        ],
    ];
    assert_eq!(expected, super::to_quartic_vec(vector));
}

#[test]
fn transpose() {
    let vector: Vec<FieldElement> = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        .into_iter()
        .map(FieldElement::from)
        .collect();
    let expected: Vec<[FieldElement; 4]> = vec![
        [
            FieldElement::from(1u8),
            FieldElement::from(5u8),
            FieldElement::from(9u8),
            FieldElement::from(13u8),
        ],
        [
            FieldElement::from(2u8),
            FieldElement::from(6u8),
            FieldElement::from(10u8),
            FieldElement::from(14u8),
        ],
        [
            FieldElement::from(3u8),
            FieldElement::from(7u8),
            FieldElement::from(11u8),
            FieldElement::from(15u8),
        ],
        [
            FieldElement::from(4u8),
            FieldElement::from(8u8),
            FieldElement::from(12u8),
            FieldElement::from(16u8),
        ],
    ];
    assert_eq!(expected, super::transpose(&vector, 1));

    let expected: Vec<[FieldElement; 4]> = vec![
        [
            FieldElement::from(1u8),
            FieldElement::from(5u8),
            FieldElement::from(9u8),
            FieldElement::from(13u8),
        ],
        [
            FieldElement::from(3u8),
            FieldElement::from(7u8),
            FieldElement::from(11u8),
            FieldElement::from(15u8),
        ],
    ];
    assert_eq!(expected, super::transpose(&vector, 2));
}
