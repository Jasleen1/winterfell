use super::{AsBytes, FieldElement, FromVec, StarkField};
use crate::utils;
use core::{
    convert::{TryFrom, TryInto},
    fmt::{Debug, Display, Formatter},
    ops::{Add, Div, Mul, Neg, Range, Sub},
    slice,
};
use rand::{distributions::Uniform, prelude::*};
use serde::{Deserialize, Serialize};
use super::super::fft;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

// Field modulus = 13
const M: u128 = 17;
// 16th root of unity
const G: u128 = 3;

const RANGE: Range<u128> = Range { start: 0, end: M };

// Number of bytes needed to represent field element
const ELEMENT_BYTES: usize = std::mem::size_of::<u128>();

// FIELD ELEMENT
// ================================================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SmallFieldElement17(u128);

impl SmallFieldElement17 {
    /// Creates a new field element from a u128 value. If the value is greater than or equal to
    /// the field modulus, modular reduction is silently preformed. This function can also be used
    /// to initialize constants.
    /// TODO: move into StarkField trait?
    pub const fn new(value: u128) -> Self {
        SmallFieldElement17(if value < M { value } else { value - M })
    }

    /// Returns filed element converted to u128 representation.
    pub fn as_u128(&self) -> u128 {
        self.0
    }

    pub fn get_twiddles(domain_size: usize) -> Vec<Self> {
        debug_assert!(
            domain_size.is_power_of_two(),
            "domain size must be a power of 2"
        );
        let root = Self::get_root_of_unity(domain_size.try_into().unwrap());
        let mut twiddles = Self::get_power_series(root, domain_size / 2);
        fft::permute(&mut twiddles);
        twiddles
    }

    pub fn get_inv_twiddles(domain_size: usize) -> Vec<Self> {
        debug_assert!(
            domain_size.is_power_of_two(),
            "domain size must be a power of 2"
        );
        let root = Self::get_root_of_unity(domain_size.try_into().unwrap());
        let inv_root = Self::exp(root, (domain_size as u32 - 1).into());
        let mut inv_twiddles = Self::get_power_series(inv_root, domain_size / 2);
        fft::permute(&mut inv_twiddles);
        inv_twiddles
    }
    

}

impl FieldElement for SmallFieldElement17 {
    type PositiveInteger = u128;

    const ZERO: Self = SmallFieldElement17(0);
    const ONE: Self = SmallFieldElement17(1);

    const ELEMENT_BYTES: usize = ELEMENT_BYTES;

    fn inv(self) -> Self {
        SmallFieldElement17(inv(self.0))
    }

    /// This implementation is about 5% faster than the one in the trait.
    fn get_power_series(b: Self, n: usize) -> Vec<Self> {
        let mut result = utils::uninit_vector(n);
        result[0] = SmallFieldElement17::ONE;
        for i in 1..result.len() {
            result[i] = result[i - 1] * b;
        }
        result
    }

    fn rand() -> Self {
        let range = Uniform::from(RANGE);
        let mut g = thread_rng();
        SmallFieldElement17(g.sample(range))
    }

    fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
        Self::try_from(bytes).ok()
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl StarkField for SmallFieldElement17 {
    /// sage: MODULUS = 13
    /// sage: GF(MODULUS).is_prime_field()
    /// True
    /// sage: GF(MODULUS).order()
    /// 340282366920938463463374557953744961537
    const MODULUS: Self::PositiveInteger = M;
    const MODULUS_BITS: u32 = 5;

    /// sage: GF(MODULUS).primitive_element()
    /// 3
    const GENERATOR: Self = SmallFieldElement17(3);

    /// sage: is_odd((MODULUS - 1) / 2^40)
    /// True
    const TWO_ADICITY: u32 = 4;

    /// sage: k = (MODULUS - 1) / 2^40
    /// sage: GF(MODULUS).primitive_element()^k
    /// 23953097886125630542083529559205016746
    const TWO_ADIC_ROOT_OF_UNITY: Self = SmallFieldElement17(G);

    fn get_root_of_unity(n: u32) -> Self {
        let small_field_size_128 = Self::MODULUS - 1;
        let small_field_size: u32 = small_field_size_128.try_into().unwrap();
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
        Self::exp(Self::GENERATOR, power.into())
    }

    fn prng_vector(seed: [u8; 32], n: usize) -> Vec<Self> {
        let range = Uniform::from(RANGE);
        let g = StdRng::from_seed(seed);
        g.sample_iter(range).take(n).map(SmallFieldElement17).collect()
    }

    fn from_int(value: u128) -> Self {
        SmallFieldElement17::new(value)
    }
}

impl FromVec<SmallFieldElement17> for SmallFieldElement17 {}

impl Display for SmallFieldElement17 {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// OVERLOADED OPERATORS
// ================================================================================================

impl Add for SmallFieldElement17 {
    type Output = SmallFieldElement17;

    fn add(self, rhs: SmallFieldElement17) -> SmallFieldElement17 {
        SmallFieldElement17(add(self.0, rhs.0))
    }
}

impl Sub for SmallFieldElement17 {
    type Output = SmallFieldElement17;

    fn sub(self, rhs: SmallFieldElement17) -> SmallFieldElement17 {
        SmallFieldElement17(sub(self.0, rhs.0))
    }
}

impl Mul for SmallFieldElement17 {
    type Output = SmallFieldElement17;

    fn mul(self, rhs: SmallFieldElement17) -> SmallFieldElement17 {
        SmallFieldElement17(mul(self.0, rhs.0))
    }
}

impl Div for SmallFieldElement17 {
    type Output = SmallFieldElement17;

    fn div(self, rhs: SmallFieldElement17) -> SmallFieldElement17 {
        SmallFieldElement17(mul(self.0, inv(rhs.0)))
    }
}

impl Neg for SmallFieldElement17 {
    type Output = SmallFieldElement17;

    fn neg(self) -> SmallFieldElement17 {
        Self(sub(0, self.0))
    }
}

// TYPE CONVERSIONS
// ================================================================================================

impl From<u128> for SmallFieldElement17 {
    /// Converts a 128-bit value into a filed element. If the value is greater than or equal to
    /// the field modulus, modular reduction is silently preformed.
    fn from(value: u128) -> Self {
        SmallFieldElement17::new(value)
    }
}

impl From<u64> for SmallFieldElement17 {
    /// Converts a 64-bit value into a filed element.
    fn from(value: u64) -> Self {
        SmallFieldElement17(value as u128)
    }
}

impl From<u32> for SmallFieldElement17 {
    /// Converts a 32-bit value into a filed element.
    fn from(value: u32) -> Self {
        SmallFieldElement17(value as u128)
    }
}

impl From<u16> for SmallFieldElement17 {
    /// Converts a 16-bit value into a filed element.
    fn from(value: u16) -> Self {
        SmallFieldElement17(value as u128)
    }
}

impl From<u8> for SmallFieldElement17 {
    /// Converts an 8-bit value into a filed element.
    fn from(value: u8) -> Self {
        SmallFieldElement17(value as u128)
    }
}

impl From<[u8; 16]> for SmallFieldElement17 {
    /// Converts the value encoded in an array of 16 bytes into a field element. The bytes
    /// are assumed to be in little-endian byte order. If the value is greater than or equal
    /// to the field modulus, modular reduction is silently preformed.
    fn from(bytes: [u8; 16]) -> Self {
        let value = u128::from_le_bytes(bytes);
        SmallFieldElement17::from(value)
    }
}

impl<'a> TryFrom<&'a [u8]> for SmallFieldElement17 {
    type Error = String;

    /// Converts a slice of bytes into a field element; returns error if the value encoded in bytes
    /// is not a valid field element. The bytes are assumed to be in little-endian byte order.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let value = bytes
            .try_into()
            .map(u128::from_le_bytes)
            .map_err(|error| format!("{}", error))?;
        if value >= M {
            return Err(format!(
                "cannot convert bytes into a field element: \
                value {} is greater or equal to the field modulus",
                value
            ));
        }
        Ok(SmallFieldElement17(value))
    }
}

impl AsBytes for SmallFieldElement17 {
    fn as_bytes(&self) -> &[u8] {
        // TODO: take endianness into account
        let self_ptr: *const SmallFieldElement17 = self;
        unsafe { slice::from_raw_parts(self_ptr as *const u8, ELEMENT_BYTES) }
    }
}

impl AsBytes for [SmallFieldElement17] {
    fn as_bytes(&self) -> &[u8] {
        // TODO: take endianness into account
        unsafe { slice::from_raw_parts(self.as_ptr() as *const u8, self.len() * ELEMENT_BYTES) }
    }
}

impl AsBytes for [SmallFieldElement17; 4] {
    fn as_bytes(&self) -> &[u8] {
        // TODO: take endianness into account
        unsafe { slice::from_raw_parts(self.as_ptr() as *const u8, self.len() * ELEMENT_BYTES) }
    }
}

// FINITE FIELD ARITHMETIC
// ================================================================================================

/// Computes (a + b) % m; a and b are assumed to be valid field elements.
fn add(a: u128, b: u128) -> u128 {
    let z = M - b;
    if a < z {
        M - z + a
    } else {
        a - z
    }
}

/// Computes (a - b) % m; a and b are assumed to be valid field elements.
fn sub(a: u128, b: u128) -> u128 {
    if a < b {
        M - b + a
    } else {
        a - b
    }
}

/// Computes (a * b) % m; a and b are assumed to be valid field elements.
fn mul(a: u128, b: u128) -> u128 {
    (a * b) % M
}



/// Computes y such that (x * y) % m = 1 except for when when x = 0; in such a case,
/// 0 is returned; x is assumed to be a valid field element.
fn inv(x: u128) -> u128 {
    if x == 0 {
        return 0;
    };
    let (_, a) = extended_euclidean(M, x);
    a % M
}

fn extended_euclidean(x: u128, y: u128) -> (u128, u128) {
    if y == 0 {
        return (1, 0);
    }
    let (u1, v1) = extended_euclidean(y, x%y);
    // let q: i128 = {(u1 - v1 * (x/y)) as i128} + {M as i128};
    // let q_mod_M = q % {M as i128}; 
    let subtracting_term = v1*(x/y);
    let second_term = (M + u1 - subtracting_term) % M;
    (v1, second_term)
    // (v1, (M + u1) - v1 * (x/y))
}


