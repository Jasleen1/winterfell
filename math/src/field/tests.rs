use std::convert::TryInto;
use num_bigint::{ BigUint };

#[test]
fn add() {
    // identity
    let r: u128 = super::rand();
    assert_eq!(r, super::add(r, 0));

    // test addition within bounds
    assert_eq!(5, super::add(2, 3));

    // test overflow
    let m: u128 = super::MODULUS;
    let t = m - 1;
    assert_eq!(0, super::add(t, 1));
    assert_eq!(1, super::add(t, 2));

    // test random values
    let r1: u128 = super::rand();
    let r2: u128 = super::rand();

    let expected = (BigUint::from(r1) + BigUint::from(r2)) % BigUint::from(super::M);
    let expected = u128::from_le_bytes((expected.to_bytes_le()[..]).try_into().unwrap());
    assert_eq!(expected, super::add(r1, r2));
}

#[test]
fn sub() {
    // identity
    let r: u128 = super::rand();
    assert_eq!(r, super::sub(r, 0));

    // test subtraction within bounds
    assert_eq!(2, super::sub(5u128, 3));

    // test underflow
    let m: u128 = super::MODULUS;
    assert_eq!(m - 2, super::sub(3u128, 5));
}

#[test]
fn mul() {
    // identity
    let r: u128 = super::rand();
    assert_eq!(0, super::mul(r, 0));
    assert_eq!(r, super::mul(r, 1));

    // test multiplication within bounds
    assert_eq!(15, super::mul(5u128, 3));

    // test overflow
    let m: u128 = super::MODULUS;
    let t = m - 1;
    assert_eq!(1, super::mul(t, t));
    assert_eq!(m - 2, super::mul(t, 2));
    assert_eq!(m - 4, super::mul(t, 4));

    let t = (m + 1) / 2;
    assert_eq!(1, super::mul(t, 2));

    // test random values
    let v1: Vec<u128> = super::rand_vector(1000);
    let v2: Vec<u128> = super::rand_vector(1000);
    for i in 0..v1.len() {
        let r1 = v1[i];
        let r2 = v2[i];

        let result = (BigUint::from(r1) * BigUint::from(r2)) % BigUint::from(super::M);
        let result = result.to_bytes_le();
        let mut expected = [0u8; 16];
        expected[0..result.len()].copy_from_slice(&result);
        let expected = u128::from_le_bytes(expected);

        if expected != super::mul(r1, 32) {
            println!("failed for: {} * {}", r1, r2);
            assert_eq!(expected, super::mul(r1, r2));
        }
    }
}

#[test]
fn inv() {
    // identity
    assert_eq!(1, super::inv(1));
    assert_eq!(0, super::inv(0));

    // test random values
    let x: Vec<u128> = super::rand_vector(1000);
    for i in 0..x.len() {
        let y = super::inv(x[i]);
        assert_eq!(1, super::mul(x[i], y));
    }
}

#[test]
fn get_root_of_unity() {
    let root_40: u128 = super::get_root_of_unity(usize::pow(2, 40));
    assert_eq!(23953097886125630542083529559205016746, root_40);
    assert_eq!(1, super::exp(root_40, u128::pow(2, 40)));

    let root_39: u128 = super::get_root_of_unity(usize::pow(2, 39));
    let expected = super::exp(root_40, 2);
    assert_eq!(expected, root_39);
    assert_eq!(1, super::exp(root_39, u128::pow(2, 39)));
}
