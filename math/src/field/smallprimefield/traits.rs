use super::{AsBytes, FieldElement, FromVec, StarkField};
use crate::{field, utils};
use core::{
    convert::{TryFrom, TryInto},
    fmt::{Debug, Display, Formatter},
    ops::{Add, Div, Mul, Neg, Range, Sub},
    slice,
};
use field::SmallPrimeFieldElement;
use rand::{distributions::Uniform, prelude::*};
use serde::{Deserialize, Serialize};

pub trait SmallPrimeField: FieldElement {
    fn get_modulus(&self) -> Self::PositiveInteger;
}


pub fn get_prime_field_root_of_unity<E: StarkField>(n: u32, modulus: u64) -> E {
    let small_field_size_64 = modulus - 1;
    let small_field_size: u32 = small_field_size_64.try_into().unwrap();
    assert!(n != 0, "cannot get root of unity for n = 0");
    assert!(
        n <= small_field_size,
        "order cannot exceed {}",
        small_field_size
    );
    assert!(
        small_field_size % n == 0,
        "Order invalid for field size {}",
        small_field_size
    );
    let power = small_field_size/n;
    E::exp(E::GENERATOR, power.into())
}

// pub trait SmallPrimeFieldInstance: FieldElement + StarkField {
//     fn get_modulus(&self) -> u64; 
//     fn get_val(&self) -> u64;
//     fn from_smallprimefield(SmallPrimeFieldElement { value, modulus }: SmallPrimeFieldElement) -> Self {

//     }
//     fn get_smallprimefield(&self) -> SmallPrimeFieldElement {
//         SmallPrimeFieldElement::new(self.get_val(), self.get_modulus())
//     }
// }



// pub trait SmallPrimeField: StarkField + FieldElement {
//     const RANGE: Range<u128>;
//     /// Prime modulus of the field. Must be of the form k * 2^n + 1 (a Proth prime).
//     /// This ensures that the field has high 2-adicity.
//     // const MODULUS: Self::PositiveInteger;

//     // /// The number of bits needed to represents `Self::MODULUS`.
//     // const MODULUS_BITS: u32;

//     // /// A multiplicative generator of the field.
//     // const GENERATOR: Self;

//     // /// Let Self::MODULUS = k * 2^n + 1; then, TWO_ADICITY is n.
//     // const TWO_ADICITY: u32;

//     /// Let Self::MODULUS = k * 2^n + 1; then, TWO_ADIC_ROOT_OF_UNITY is 2^n root of unity
//     /// computed as Self::GENERATOR^k.
//     // const TWO_ADIC_ROOT_OF_UNITY: Self;
    
//     fn new(value: u128) -> Self;
//     fn inv(self) -> Self;

//     /// This implementation is about 5% faster than the one in the trait.
//     fn get_power_series(b: Self, n: usize) -> Vec<Self> {
//         let mut result = utils::uninit_vector(n);
//         result[0] = Self::ONE;
//         for i in 1..result.len() {
//             result[i] = result[i - 1] * b;
//         }
//         result
//     }

//     fn rand() -> Self {
//         let range = Uniform::from(Self::RANGE);
//         let mut g = thread_rng();
//         Self::new(g.sample(range))
//     }

//     fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
//         Self::try_from(bytes).ok()
//     }

//     fn to_bytes(&self) -> Vec<u8> {
//         self.as_bytes().to_vec()
//     }

//     fn from_int(value: u128) -> Self {
//         Self::new(value)
//     }

//     fn prng_vector(seed: [u8; 32], n: usize) -> Vec<Self> {
//         let range = Uniform::from(Self::RANGE);
//         let g = StdRng::from_seed(seed);
//         g.sample_iter(range).take(n).map(Self::new).collect()
//     }
    
// }