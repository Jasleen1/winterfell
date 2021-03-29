use super::{AsBytes, BaseElement, FieldElement, FromVec};
use core::{
    convert::TryFrom,
    fmt::{Debug, Display, Formatter},
    ops::{Add, Div, Mul, Neg, Sub},
    slice,
};
use serde::{Deserialize, Serialize};

// QUADRATIC EXTENSION FIELD
// ================================================================================================

/// Represents an element in a quadratic extensions of the base field. The extension element
/// is α + β * φ, where φ is a root of the polynomial x^2 - x - 1, and α and β are base
/// field elements. In other words, the extension field is F[X]/(X^2-X-1).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct QuadElement(BaseElement, BaseElement);

impl FieldElement for QuadElement {
    type PositiveInteger = u128;
    type Base = BaseElement;

    const ELEMENT_BYTES: usize = BaseElement::ELEMENT_BYTES * 2;
    const ZERO: Self = Self(BaseElement::ZERO, BaseElement::ZERO);
    const ONE: Self = Self(BaseElement::ONE, BaseElement::ZERO);

    fn inv(self) -> Self {
        if self == Self::ZERO {
            return Self::ZERO;
        }
        #[allow(clippy::suspicious_operation_groupings)]
        let denom = (self.0 * self.0) + (self.0 * self.1) - (self.1 * self.1);
        let denom_inv = denom.inv();
        Self((self.0 + self.1) * denom_inv, self.1.neg() * denom_inv)
    }

    fn conjugate(&self) -> Self {
        Self(self.0 + self.1, -self.1)
    }

    fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
        Self::try_from(&bytes[..Self::ELEMENT_BYTES as usize]).ok()
    }

    fn rand() -> Self {
        Self(BaseElement::rand(), BaseElement::rand())
    }

    fn zeroed_vector(n: usize) -> Vec<Self> {
        // this uses a specialized vector initialization code which requests zero-filled memory
        // from the OS; unfortunately, this works only for built-in types and we can't use
        // Self::ZERO here as much less efficient initialization procedure will be invoked.
        let result = vec![0u8; n * Self::ELEMENT_BYTES];

        // so, now we need to translate a zero-filled vector of bytes into a vector of field
        // elements
        let mut v = std::mem::ManuallyDrop::new(result);
        let p = v.as_mut_ptr();
        let len = v.len() / Self::ELEMENT_BYTES;
        let cap = v.capacity() / Self::ELEMENT_BYTES;
        unsafe { Vec::from_raw_parts(p as *mut Self, len, cap) }
    }

    fn elements_as_bytes(elements: &[Self]) -> &[u8] {
        elements.as_bytes()
    }
}

impl Display for QuadElement {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

// OVERLOADED OPERATORS
// ================================================================================================

impl Add for QuadElement {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl Sub for QuadElement {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl Mul for QuadElement {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let coef0_mul = self.0 * rhs.0;
        Self(
            coef0_mul + self.1 * rhs.1,
            (self.0 + self.1) * (rhs.0 + rhs.1) - coef0_mul,
        )
    }
}

impl Div for QuadElement {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self {
        self * rhs.inv()
    }
}

impl Neg for QuadElement {
    type Output = Self;

    fn neg(self) -> Self {
        Self(BaseElement::ZERO - self.0, BaseElement::ZERO - self.1)
    }
}

// TYPE CONVERSIONS
// ================================================================================================

impl From<BaseElement> for QuadElement {
    fn from(e: BaseElement) -> Self {
        Self(e, BaseElement::ZERO)
    }
}

impl From<u128> for QuadElement {
    fn from(value: u128) -> Self {
        QuadElement(BaseElement::from(value), BaseElement::ZERO)
    }
}

impl From<u64> for QuadElement {
    fn from(value: u64) -> Self {
        QuadElement(BaseElement::from(value), BaseElement::ZERO)
    }
}

impl From<u32> for QuadElement {
    fn from(value: u32) -> Self {
        QuadElement(BaseElement::from(value), BaseElement::ZERO)
    }
}

impl From<u16> for QuadElement {
    fn from(value: u16) -> Self {
        QuadElement(BaseElement::from(value), BaseElement::ZERO)
    }
}

impl From<u8> for QuadElement {
    fn from(value: u8) -> Self {
        QuadElement(BaseElement::from(value), BaseElement::ZERO)
    }
}

impl<'a> TryFrom<&'a [u8]> for QuadElement {
    type Error = String;

    /// Converts a slice of bytes into a field element; returns error if the value encoded in bytes
    /// is not a valid field element. The bytes are assumed to be in little-endian byte order.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < Self::ELEMENT_BYTES {
            return Err(
                "need more bytes in order to convert into extension field element".to_string(),
            );
        }
        let value0 = match BaseElement::try_from(&bytes[..BaseElement::ELEMENT_BYTES]) {
            Ok(val) => val,
            Err(_) => {
                return Err("could not convert into field element".to_string());
            }
        };
        let value1 = match BaseElement::try_from(&bytes[BaseElement::ELEMENT_BYTES..]) {
            Ok(val) => val,
            Err(_) => {
                return Err("could not convert into field element".to_string());
            }
        };
        Ok(Self(value0, value1))
    }
}

impl FromVec<BaseElement> for QuadElement {}

// SERIALIZATION
// ================================================================================================

impl AsBytes for QuadElement {
    fn as_bytes(&self) -> &[u8] {
        // TODO: take endianness into account
        let self_ptr: *const QuadElement = self;
        unsafe { slice::from_raw_parts(self_ptr as *const u8, Self::ELEMENT_BYTES) }
    }
}

impl AsBytes for [QuadElement] {
    fn as_bytes(&self) -> &[u8] {
        // TODO: take endianness into account
        unsafe {
            slice::from_raw_parts(
                self.as_ptr() as *const u8,
                self.len() * QuadElement::ELEMENT_BYTES,
            )
        }
    }
}

impl<const N: usize> AsBytes for [QuadElement; N] {
    fn as_bytes(&self) -> &[u8] {
        // TODO: take endianness into account
        unsafe {
            slice::from_raw_parts(
                self.as_ptr() as *const u8,
                self.len() * QuadElement::ELEMENT_BYTES,
            )
        }
    }
}
