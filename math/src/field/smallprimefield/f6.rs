use super::{AsBytes, FieldElement, FromVec, SmallPrimeFieldElement, StarkField, traits::SmallPrimeField};
use crate::utils;
use core::{
    convert::{TryFrom, TryInto},
    fmt::{Debug, Display, Formatter},
    ops::{Add, Div, Mul, Neg, Range, Sub},
    slice,
};
use rand::{distributions::Uniform, prelude::*};
use serde::{Deserialize, Serialize};



// CONSTANTS
// ================================================================================================

// Field modulus = 37
const M: u64 = 37;
// 36th root of unity
const G: u64 = 2;

const RANGE: Range<u64> = Range { start: 0, end: M };

// Number of bytes needed to represent field element
const ELEMENT_BYTES: usize = std::mem::size_of::<u64>();

// FIELD ELEMENT
// ================================================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SmallFieldElement37(u64);

impl SmallFieldElement37 {
    /// Creates a new field element from a u128 value. If the value is greater than or equal to
    /// the field modulus, modular reduction is silently preformed. This function can also be used
    /// to initialize constants.
    /// TODO: move into StarkField trait?
    pub const fn new(value: u64) -> Self {
        SmallFieldElement37(if value < M { value } else { value - M })
    }

    /// Returns field element converted to u64 representation.
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    // Returns a SmallFieldElement37 representation from a SmallPrimeFieldElement
    pub fn from_small_prime_field_elt(elt: SmallPrimeFieldElement) -> Self {
        assert!{elt.get_modulus() == M, "SmallPrimeFieldElement modulus = {}, is not the same as {}", elt.get_modulus(), M};
        SmallFieldElement37::new(elt.get_val())
    } 

    pub fn to_small_prime_field_elt(elt: Self) -> SmallPrimeFieldElement {
        SmallPrimeFieldElement{value: elt.0, modulus: M}
    }
}

impl FieldElement for SmallFieldElement37 {
    type PositiveInteger = u64;

    const ZERO: Self = SmallFieldElement37(0);
    const ONE: Self = SmallFieldElement37(1);

    const ELEMENT_BYTES: usize = ELEMENT_BYTES;

    fn inv(self) -> Self {
        Self::from_small_prime_field_elt(Self::to_small_prime_field_elt(self).inv())
    }

    /// This implementation is about 5% faster than the one in the trait.
    fn get_power_series(b: Self, n: usize) -> Vec<Self> {
        let mut result = utils::uninit_vector(n);
        result[0] = SmallFieldElement37::ONE;
        for i in 1..result.len() {
            result[i] = result[i - 1] * b;
        }
        result
    }

    fn rand() -> Self {
        let range = Uniform::from(RANGE);
        let mut g = thread_rng();
        SmallFieldElement37(g.sample(range))
    }

    fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
        Self::try_from(bytes).ok()
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl StarkField for SmallFieldElement37 {
    /// sage: MODULUS = 37
    /// sage: GF(MODULUS).is_prime_field()
    /// True
    /// sage: GF(MODULUS).order()
    /// 340282366920938463463374557953744961537
    const MODULUS: Self::PositiveInteger = M;
    const MODULUS_BITS: u32 = 6;

    /// sage: GF(MODULUS).primitive_element()
    /// 3
    const GENERATOR: Self = SmallFieldElement37(2);

    /// sage: is_odd((MODULUS - 1) / 2^40)
    /// True
    const TWO_ADICITY: u32 = 2;

    /// sage: k = (MODULUS - 1) / 2^40
    /// sage: GF(MODULUS).primitive_element()^k
    /// 23953097886125630542083529559205016746
    const TWO_ADIC_ROOT_OF_UNITY: Self = SmallFieldElement37(G);

    fn get_root_of_unity(n: u32) -> Self {
        super::traits::get_prime_field_root_of_unity(n, Self::MODULUS)
        // let small_field_size_64 = Self::MODULUS - 1;
        // let small_field_size: u32 = small_field_size_64.try_into().unwrap();
        // assert!(n != 0, "cannot get root of unity for n = 0");
        // assert!(
        //     n <= small_field_size,
        //     "order cannot exceed {}",
        //     small_field_size
        // );
        // assert!(
        //     small_field_size % n == 0,
        //     "Order invalid for field size {}",
        //     small_field_size
        // );
        // let power = small_field_size/n;
        // Self::exp(Self::GENERATOR, power.into())
    }

    fn prng_vector(seed: [u8; 32], n: usize) -> Vec<Self> {
        let range = Uniform::from(RANGE);
        let g = StdRng::from_seed(seed);
        g.sample_iter(range).take(n).map(SmallFieldElement37::new).collect()
    }

    fn from_int(value: u64) -> Self {
        SmallFieldElement37::new(value)
    }
}

impl FromVec<SmallFieldElement37> for SmallFieldElement37 {}

impl Display for SmallFieldElement37 {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// OVERLOADED OPERATORS
// ================================================================================================

impl Add for SmallFieldElement37 {
    type Output = SmallFieldElement37;

    fn add(self, rhs: SmallFieldElement37) -> SmallFieldElement37 {
        Self::from_small_prime_field_elt(Self::to_small_prime_field_elt(self) + Self::to_small_prime_field_elt(rhs))
    }
}

impl Sub for SmallFieldElement37 {
    type Output = SmallFieldElement37;

    fn sub(self, rhs: SmallFieldElement37) -> SmallFieldElement37 {
        Self::from_small_prime_field_elt(Self::to_small_prime_field_elt(self) - Self::to_small_prime_field_elt(rhs))
    }
}

impl Mul for SmallFieldElement37 {
    type Output = SmallFieldElement37;

    fn mul(self, rhs: SmallFieldElement37) -> SmallFieldElement37 {
        Self::from_small_prime_field_elt(Self::to_small_prime_field_elt(self) * Self::to_small_prime_field_elt(rhs))
    }
}

impl Div for SmallFieldElement37 {
    type Output = SmallFieldElement37;

    fn div(self, rhs: SmallFieldElement37) -> SmallFieldElement37 {
        Self::from_small_prime_field_elt(Self::to_small_prime_field_elt(self) / Self::to_small_prime_field_elt(rhs))
    }
}

impl Neg for SmallFieldElement37 {
    type Output = SmallFieldElement37;

    fn neg(self) -> SmallFieldElement37 {
        Self::from_small_prime_field_elt(- Self::to_small_prime_field_elt(self))
    }
}

// TYPE CONVERSIONS
// ================================================================================================

impl From<u128> for SmallFieldElement37 {
    /// Converts a 128-bit value into a field element. If the value is greater than or equal to
    /// the field modulus, modular reduction is silently preformed.
    fn from(value: u128) -> Self {
        SmallFieldElement37(value as u64)
    }
}

impl From<u64> for SmallFieldElement37 {
    /// Converts a 64-bit value into a field element.
    fn from(value: u64) -> Self {
        SmallFieldElement37(value as u64)
    }
}

impl From<u32> for SmallFieldElement37 {
    /// Converts a 32-bit value into a field element.
    fn from(value: u32) -> Self {
        SmallFieldElement37(value as u64)
    }
}

impl From<u16> for SmallFieldElement37 {
    /// Converts a 16-bit value into a field element.
    fn from(value: u16) -> Self {
        SmallFieldElement37(value as u64)
    }
}

impl From<u8> for SmallFieldElement37 {
    /// Converts an 8-bit value into a field element.
    fn from(value: u8) -> Self {
        SmallFieldElement37(value as u64)
    }
}

impl From<[u8; 8]> for SmallFieldElement37 {
    /// Converts the value encoded in an array of 8 bytes into a field element. The bytes
    /// are assumed to be in little-endian byte order. If the value is greater than or equal
    /// to the field modulus, modular reduction is silently preformed.
    fn from(bytes: [u8; 8]) -> Self {
        let value = u64::from_le_bytes(bytes);
        SmallFieldElement37::from(value)
    }
}

impl<'a> TryFrom<&'a [u8]> for SmallFieldElement37 {
    type Error = String;

    /// Converts a slice of bytes into a field element; returns error if the value encoded in bytes
    /// is not a valid field element. The bytes are assumed to be in little-endian byte order.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let value = bytes
            .try_into()
            .map(u64::from_le_bytes)
            .map_err(|error| format!("{}", error))?;
        if value >= M {
            return Err(format!(
                "cannot convert bytes into a field element: \
                value {} is greater or equal to the field modulus",
                value
            ));
        }
        Ok(SmallFieldElement37(value))
    }
}

impl AsBytes for SmallFieldElement37 {
    fn as_bytes(&self) -> &[u8] {
        // TODO: take endianness into account
        let self_ptr: *const SmallFieldElement37 = self;
        unsafe { slice::from_raw_parts(self_ptr as *const u8, ELEMENT_BYTES) }
    }
}

impl AsBytes for [SmallFieldElement37] {
    fn as_bytes(&self) -> &[u8] {
        // TODO: take endianness into account
        unsafe { slice::from_raw_parts(self.as_ptr() as *const u8, self.len() * ELEMENT_BYTES) }
    }
}

impl AsBytes for [SmallFieldElement37; 4] {
    fn as_bytes(&self) -> &[u8] {
        // TODO: take endianness into account
        unsafe { slice::from_raw_parts(self.as_ptr() as *const u8, self.len() * ELEMENT_BYTES) }
    }
}




