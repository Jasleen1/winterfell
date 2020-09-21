use crate::{field, polynom};

#[test]
fn eval() {
    let x: u128 = 11269864713250585702;
    let poly: [u128; 4] = [
        384863712573444386,
        7682273369345308472,
        13294661765012277990,
        16234810094004944758,
    ];
    assert_eq!(polynom::eval(&poly, x), super::eval(&poly, x));
}

#[test]
fn interpolate_batch() {
    let r = field::get_root_of_unity(16);
    let xs = super::to_quartic_vec(field::get_power_series(r, 16));
    let ys = super::to_quartic_vec(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);

    let mut expected: Vec<[u128; 4]> = vec![];
    for i in 0..xs.len() {
        let mut row = [0u128; 4];
        row.copy_from_slice(&polynom::interpolate(&xs[i], &ys[i], false));
        expected.push(row);
    }

    assert_eq!(expected, super::interpolate_batch(&xs, &ys));
}

#[test]
fn evaluate_batch() {
    let x = field::rand();
    let polys: [[u128; 4]; 4] = [
        [
            7956382178997078105,
            6172178935026293282,
            5971474637801684060,
            16793452009046991148,
        ],
        [
            7956382178997078109,
            15205743380705406848,
            12475269242634339237,
            194846859619262948,
        ],
        [
            7956382178997078113,
            12274564945409730015,
            5971474637801684060,
            1653291871389032149,
        ],
        [
            7956382178997078117,
            3241000499730616449,
            12475269242634339237,
            18251897020816760349,
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
    let vector = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let expected: Vec<[u128; 4]> = vec![
        [1, 2, 3, 4],
        [5, 6, 7, 8],
        [9, 10, 11, 12],
        [13, 14, 15, 16],
    ];
    assert_eq!(expected, super::to_quartic_vec(vector));
}

#[test]
fn transpose() {
    let vector = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let expected: Vec<[u128; 4]> = vec![
        [1, 5, 9, 13],
        [2, 6, 10, 14],
        [3, 7, 11, 15],
        [4, 8, 12, 16],
    ];
    assert_eq!(expected, super::transpose(&vector, 1));

    let expected: Vec<[u128; 4]> = vec![[1, 5, 9, 13], [3, 7, 11, 15]];
    assert_eq!(expected, super::transpose(&vector, 2));
}
