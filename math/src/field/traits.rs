use crate::utils;
use core::{
    convert::TryFrom,
    fmt::{Debug, Display},
    ops::{
        Add, AddAssign, BitAnd, Div, DivAssign, Mul, MulAssign, Neg, Shl, ShrAssign, Sub, SubAssign,
    },
};

#[cfg(feature = "concurrent")]
use rayon::prelude::*;

// FIELD ELEMENT
// ================================================================================================

pub trait FieldElement:
    AsBytes
    + Copy
    + Clone
    + Debug
    + Display
    + Default
    + Send
    + Sync
    + Eq
    + PartialEq
    + Sized
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Mul<Self, Output = Self>
    + Div<Self, Output = Self>
    + AddAssign<Self>
    + SubAssign<Self>
    + MulAssign<Self>
    + DivAssign<Self>
    + Neg<Output = Self>
    + From<u128>
    + From<u64>
    + From<u32>
    + From<u16>
    + From<u8>
    + for<'a> TryFrom<&'a [u8]>
{
    type PositiveInteger: BitAnd<Output = Self::PositiveInteger>
        + PartialEq
        + PartialOrd
        + ShrAssign
        + Shl<u32, Output = Self::PositiveInteger>
        + From<u32>
        + From<u64>
        + Copy;

    type Base: StarkField;

    /// Number of bytes needed to encode an element
    const ELEMENT_BYTES: usize;

    /// The additive identity.
    const ZERO: Self;

    /// The multiplicative identity.
    const ONE: Self;

    // ALGEBRA
    // --------------------------------------------------------------------------------------------

    /// Exponentiates this element by `power` parameter.
    fn exp(self, power: Self::PositiveInteger) -> Self {
        let mut r = Self::ONE;
        let mut b = self;
        let mut p = power;

        let int_zero = Self::PositiveInteger::from(0u32);
        let int_one = Self::PositiveInteger::from(1u32);

        if b == Self::ZERO {
            return Self::ZERO;
        } else if p == int_zero {
            return Self::ONE;
        }

        // TODO: optimize
        while p > int_zero {
            if p & int_one == int_one {
                r *= b;
            }
            p >>= int_one;
            b = b * b;
        }

        r
    }

    /// Generates a vector with values [1, b, b^2, b^3, b^4, ..., b^(n-1)].
    /// When `concurrent` feature is enabled, series generation is done concurrently in multiple
    /// threads.
    fn get_power_series(b: Self, n: usize) -> Vec<Self> {
        const MIN_CONCURRENT_SIZE: usize = 1024;
        let mut result = utils::uninit_vector(n);
        if cfg!(feature = "concurrent") && n >= MIN_CONCURRENT_SIZE && n.is_power_of_two() {
            #[cfg(feature = "concurrent")]
            {
                let batch_size = n / rayon::current_num_threads().next_power_of_two();
                result
                    .par_chunks_mut(batch_size)
                    .enumerate()
                    .for_each(|(i, batch)| {
                        let batch_start = i * batch_size;
                        fill_power_series(batch, b, b.exp((batch_start as u32).into()));
                    });
            }
        } else {
            fill_power_series(&mut result, b, Self::ONE);
        }
        result
    }

    /// Generates a vector with values [s, s * b, s * b^2, s * b^3, s * b^4, ..., s * b^(n-1)].
    /// When `concurrent` feature is enabled, series generation is done concurrently in multiple
    /// threads.
    fn get_power_series_with_offset(b: Self, s: Self, n: usize) -> Vec<Self> {
        const MIN_CONCURRENT_SIZE: usize = 1024;
        let mut result = utils::uninit_vector(n);
        if cfg!(feature = "concurrent") && n >= MIN_CONCURRENT_SIZE && n.is_power_of_two() {
            #[cfg(feature = "concurrent")]
            {
                let batch_size = n / rayon::current_num_threads().next_power_of_two();
                result
                    .par_chunks_mut(batch_size)
                    .enumerate()
                    .for_each(|(i, batch)| {
                        let batch_start = i * batch_size;
                        let start = s * b.exp((batch_start as u32).into());
                        fill_power_series(batch, b, start);
                    });
            }
        } else {
            fill_power_series(&mut result, b, s);
        }
        result
    }

    /// Computes a multiplicative inverse of this element. If this element is ZERO, ZERO is
    /// returned.
    fn inv(self) -> Self;

    /// Computes a multiplicative inverse of a sequence of elements using batch inversion method.
    /// Any ZEROs in the provided sequence are ignored.
    fn inv_many(values: &[Self]) -> Vec<Self> {
        let mut result = Vec::with_capacity(values.len());
        let mut last = Self::ONE;
        for &value in values {
            result.push(last);
            if value != Self::ZERO {
                last *= value;
            }
        }

        last = last.inv();

        for i in (0..values.len()).rev() {
            if values[i] == Self::ZERO {
                result[i] = Self::ZERO;
            } else {
                result[i] = last * result[i];
                last *= values[i];
            }
        }
        result
    }

    /// Returns a conjugate of this field element.
    fn conjugate(&self) -> Self;

    // RANDOMNESS
    // --------------------------------------------------------------------------------------------

    /// Returns a cryptographically-secure random element drawn uniformly from the entire field.
    fn rand() -> Self;

    /// Returns a field element if the set of bytes forms a valid field element, otherwise returns
    /// None. This function is primarily intended for sampling random field elements from a hash
    /// function output.
    fn from_random_bytes(bytes: &[u8]) -> Option<Self>;

    // SERIALIZATION / DESERIALIZATION
    // --------------------------------------------------------------------------------------------

    /// Converts a list of elements into byte representation. The conversion just re-interprets
    /// the underlying memory and is thus zero-copy.
    fn elements_as_bytes(elements: &[Self]) -> &[u8];

    /// Reads elements from the specified slice of bytes and copies them into the provided
    /// result slice. The elements are assumed to be stored in the slice one after the other
    /// in little-endian byte order. Returns the number of read elements.
    fn read_into(bytes: &[u8], result: &mut [Self]) -> Result<usize, String> {
        let num_elements = bytes.len() / Self::ELEMENT_BYTES;
        if result.len() < num_elements {
            return Err(format!(
                "result must be at least {} elements long, but was {}",
                num_elements,
                result.len()
            ));
        }

        for i in (0..bytes.len()).step_by(Self::ELEMENT_BYTES) {
            match Self::try_from(&bytes[i..i + Self::ELEMENT_BYTES]) {
                Ok(value) => result[i / Self::ELEMENT_BYTES] = value,
                Err(_) => {
                    return Err(format!(
                        "failed to read element from bytes at position {}",
                        i
                    ))
                }
            }
        }

        Ok(num_elements)
    }

    /// Returns a vector of elements read from the provided slice of bytes. The elements are
    /// assumed to be stored in the slice one after the other in little-endian byte order.
    fn read_into_vec(bytes: &[u8]) -> Result<Vec<Self>, String> {
        if bytes.len() % Self::ELEMENT_BYTES != 0 {
            return Err(String::from(
                "number of bytes does not divide into whole number of elements",
            ));
        }

        let mut result = vec![Self::ZERO; bytes.len() / Self::ELEMENT_BYTES];
        Self::read_into(bytes, &mut result)?;
        Ok(result)
    }

    // INITIALIZATION
    // --------------------------------------------------------------------------------------------

    /// Returns a vector initialized with all zero elements; specialized implementations of this
    /// function may be faster than the generic implementation.
    fn zeroed_vector(n: usize) -> Vec<Self> {
        vec![Self::ZERO; n]
    }
}

// HELPER FUNCTIONS
// ------------------------------------------------------------------------------------------------

#[inline(always)]
fn fill_power_series<E: FieldElement>(result: &mut [E], base: E, start: E) {
    result[0] = start;
    for i in 1..result.len() {
        result[i] = result[i - 1] * base;
    }
}

// STARK FIELD
// ================================================================================================

pub trait StarkField: FieldElement + AsBytes {
    /// Prime modulus of the field. Must be of the form k * 2^n + 1 (a Proth prime).
    /// This ensures that the field has high 2-adicity.
    const MODULUS: Self::PositiveInteger;

    /// The number of bits needed to represents `Self::MODULUS`.
    const MODULUS_BITS: u32;

    /// A multiplicative generator of the field.
    const GENERATOR: Self;

    /// Let Self::MODULUS = k * 2^n + 1; then, TWO_ADICITY is n.
    const TWO_ADICITY: u32;

    /// Let Self::MODULUS = k * 2^n + 1; then, TWO_ADIC_ROOT_OF_UNITY is 2^n root of unity
    /// computed as Self::GENERATOR^k.
    const TWO_ADIC_ROOT_OF_UNITY: Self;

    /// Returns the root of unity of order 2^n. Panics if the root of unity for
    /// the specified order does not exist in this field.
    fn get_root_of_unity(n: u32) -> Self {
        assert!(n != 0, "cannot get root of unity for n = 0");
        assert!(
            n <= Self::TWO_ADICITY,
            "order cannot exceed 2^{}",
            Self::TWO_ADICITY
        );
        let power = Self::PositiveInteger::from(1u32) << (Self::TWO_ADICITY - n);
        Self::TWO_ADIC_ROOT_OF_UNITY.exp(power)
    }

    /// Returns a vector of n pseudo-random elements drawn uniformly from the entire
    /// field based on the provided seed.
    fn prng_vector(seed: [u8; 32], n: usize) -> Vec<Self>;

    // Returns an integer representation of the field element.
    fn as_int(&self) -> Self::PositiveInteger;
}

pub trait FromVec<E: FieldElement>: From<E> {
    fn from_vec(v: &[E]) -> Vec<Self>
    where
        Self: Sized,
    {
        v.iter().map(|&x| Self::from(x)).collect()
    }
}

// SERIALIZATION
// ================================================================================================

pub trait AsBytes {
    fn as_bytes(&self) -> &[u8];
}
