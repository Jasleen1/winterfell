use core::{
    convert::TryFrom,
    fmt::{Debug, Display},
    ops::{Add, BitAnd, Div, Mul, Neg, Shl, ShrAssign, Sub},
};

// FIELD ELEMENT
// ================================================================================================

pub trait FieldElement:
    Copy
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
        + Copy;

    /// Number of bytes needed to encode an element
    const ELEMENT_BYTES: usize;

    /// The additive identity.
    const ZERO: Self;

    /// The multiplicative identity.
    const ONE: Self;

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
                r = r * b;
            }
            p >>= int_one;
            b = b * b;
        }

        r
    }

    /// Generates a vector with values [1, b, b^2, b^3, b^4, ..., b^(n-1)].
    fn get_power_series(b: Self, n: usize) -> Vec<Self> {
        let mut result = vec![Self::default(); n];
        result[0] = Self::ONE;
        for i in 1..n {
            result[i] = result[i - 1] * b;
        }
        result
    }

    /// Computes a multiplicative inverse of this element. If this element is ZERO
    /// ZERO is returned.
    fn inv(self) -> Self;

    /// Computes a multiplicative inverse of a sequence of elements using batch
    /// inversion method. Any ZEROs in the provided sequence are ignored.
    fn inv_many(values: &[Self]) -> Vec<Self> {
        let mut result = Vec::with_capacity(values.len());
        let mut last = Self::ONE;
        for &value in values {
            result.push(last);
            if value != Self::ZERO {
                last = last * value;
            }
        }

        last = Self::inv(last);

        for i in (0..values.len()).rev() {
            if values[i] == Self::ZERO {
                result[i] = Self::ZERO;
            } else {
                result[i] = last * result[i];
                last = last * values[i];
            }
        }
        result
    }

    /// Returns a cryptographically-secure random element drawn uniformly from
    /// the entire field.
    fn rand() -> Self;

    /// Returns a field element if the set of bytes forms a valid field element,
    /// otherwise returns None. This function is primarily intended for sampling
    /// random field elements from a hash function output.
    fn from_random_bytes(bytes: &[u8]) -> Option<Self>;

    /// Returns the byte representation of the element in little-endian byte order.
    fn to_bytes(&self) -> Vec<u8>;

    /// Writes a vector of filed elements into the provided slice of bytes in little-endian
    /// byte order.
    fn write_into(elements: &[Self], result: &mut [u8]) -> Result<usize, String> {
        let num_bytes = elements.len() * Self::ELEMENT_BYTES;
        if result.len() < num_bytes {
            return Err(format!(
                "result must be at least {} bytes long, but was {}",
                num_bytes,
                result.len()
            ));
        }

        for (i, element) in elements.iter().enumerate() {
            let start = i * Self::ELEMENT_BYTES;
            result[start..start + Self::ELEMENT_BYTES].copy_from_slice(&element.to_bytes());
        }
        Ok(num_bytes)
    }

    /// Returns a vector of bytes with all elements from the provided slice written
    /// into the vector in little-endian byte order.
    fn write_into_vec(elements: &[Self]) -> Vec<u8> {
        let mut result = Vec::with_capacity(elements.len() * Self::ELEMENT_BYTES);
        for element in elements {
            result.extend_from_slice(&element.to_bytes());
        }
        result
    }

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
    fn read_to_vec(bytes: &[u8]) -> Result<Vec<Self>, String> {
        if bytes.len() % Self::ELEMENT_BYTES != 0 {
            return Err(String::from(
                "number of bytes does not divide into whole number of elements",
            ));
        }

        let mut result = vec![Self::ZERO; bytes.len() / Self::ELEMENT_BYTES];
        Self::read_into(bytes, &mut result)?;
        Ok(result)
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
        let power = Self::PositiveInteger::from(1) << (Self::TWO_ADICITY - n);
        Self::exp(Self::TWO_ADIC_ROOT_OF_UNITY, power)
    }

    /// Returns a vector of n pseudo-random elements drawn uniformly from the entire
    /// field based on the provided seed.
    fn prng_vector(seed: [u8; 32], n: usize) -> Vec<Self>;

    fn from_int(value: Self::PositiveInteger) -> Self;
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
