use super::*;
use num_bigint::BigUint;
use std::convert::TryInto;

#[test]
fn test_add() {
    // identity
    let r: u128 = rand();
    assert_eq!(r, add(r, 0));

    // test addition within bounds
    assert_eq!(5, add(2, 3));

    // test overflow
    let m: u128 = MODULUS;
    let t = m - 1;
    assert_eq!(0, add(t, 1));
    assert_eq!(1, add(t, 2));

    // test random values
    let r1: u128 = rand();
    let r2: u128 = rand();

    let expected = (BigUint::from(r1) + BigUint::from(r2)) % BigUint::from(M);
    let expected = u128::from_le_bytes((expected.to_bytes_le()[..]).try_into().unwrap());
    assert_eq!(expected, add(r1, r2));
}

#[test]
fn test_sub() {
    // identity
    let r: u128 = rand();
    assert_eq!(r, sub(r, 0));

    // test subtraction within bounds
    assert_eq!(2, sub(5u128, 3));

    // test underflow
    let m: u128 = MODULUS;
    assert_eq!(m - 2, sub(3u128, 5));
}

#[test]
fn test_mul() {
    // identity
    let r: u128 = rand();
    assert_eq!(0, mul(r, 0));
    assert_eq!(r, mul(r, 1));

    // test multiplication within bounds
    assert_eq!(15, mul(5u128, 3));

    // test overflow
    let m: u128 = MODULUS;
    let t = m - 1;
    assert_eq!(1, mul(t, t));
    assert_eq!(m - 2, mul(t, 2));
    assert_eq!(m - 4, mul(t, 4));

    let t = (m + 1) / 2;
    assert_eq!(1, mul(t, 2));

    // test random values
    let v1: Vec<u128> = rand_vector(1000);
    let v2: Vec<u128> = rand_vector(1000);
    for i in 0..v1.len() {
        let r1 = v1[i];
        let r2 = v2[i];

        let result = (BigUint::from(r1) * BigUint::from(r2)) % BigUint::from(M);
        let result = result.to_bytes_le();
        let mut expected = [0u8; 16];
        expected[0..result.len()].copy_from_slice(&result);
        let expected = u128::from_le_bytes(expected);

        if expected != mul(r1, 32) {
            println!("failed for: {} * {}", r1, r2);
            assert_eq!(expected, mul(r1, r2));
        }
    }
}

#[test]
fn test_inv() {
    // identity
    assert_eq!(1, inv(1));
    assert_eq!(0, inv(0));

    // test random values
    let x: Vec<u128> = rand_vector(1000);
    for i in 0..x.len() {
        let y = inv(x[i]);
        assert_eq!(1, mul(x[i], y));
    }
}

#[test]
fn test_get_root_of_unity() {
    let root_40: u128 = get_root_of_unity(usize::pow(2, 40));
    assert_eq!(23953097886125630542083529559205016746, root_40);
    assert_eq!(1, exp(root_40, u128::pow(2, 40)));

    let root_39: u128 = get_root_of_unity(usize::pow(2, 39));
    let expected = exp(root_40, 2);
    assert_eq!(expected, root_39);
    assert_eq!(1, exp(root_39, u128::pow(2, 39)));
}

#[test]
fn test_g_is_2_exp_40_root() {
    assert_eq!(exp(G, 1u128 << 40), 1u128)
}
