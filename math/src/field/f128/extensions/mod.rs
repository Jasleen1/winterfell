use super::super::{AsBytes, BaseElement, FieldElement, FromVec};
use crate::errors::SerializationError;
use core::{
    convert::TryFrom,
    fmt::{Debug, Display, Formatter},
    mem,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    slice,
};

#[cfg(test)]
mod tests;

// QUADRATIC EXTENSION FIELD
// ================================================================================================

/// Represents an element in a quadratic extensions of the base field. The extension element
/// is α + β * φ, where φ is a root of the polynomial x^2 - x - 1, and α and β are base
/// field elements. In other words, the extension field is F[X]/(X^2-X-1).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct QuadElement(BaseElement, BaseElement);

impl QuadElement {
    pub const fn new(v0: u128, v1: u128) -> Self {
        QuadElement(BaseElement::new(v0), BaseElement::new(v1))
    }
}

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

    fn rand() -> Self {
        Self(BaseElement::rand(), BaseElement::rand())
    }

    fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
        Self::try_from(&bytes[..Self::ELEMENT_BYTES as usize]).ok()
    }

    fn elements_into_bytes(elements: Vec<Self>) -> Vec<u8> {
        let mut v = std::mem::ManuallyDrop::new(elements);
        let p = v.as_mut_ptr();
        let len = v.len() * Self::ELEMENT_BYTES;
        let cap = v.capacity() * Self::ELEMENT_BYTES;
        unsafe { Vec::from_raw_parts(p as *mut u8, len, cap) }
    }

    fn elements_as_bytes(elements: &[Self]) -> &[u8] {
        elements.as_bytes()
    }

    unsafe fn bytes_as_elements(bytes: &[u8]) -> Result<&[Self], SerializationError> {
        if bytes.len() % Self::ELEMENT_BYTES != 0 {
            return Err(SerializationError::NotEnoughBytesForWholeElements(
                bytes.len(),
            ));
        }

        let p = bytes.as_ptr();
        let len = bytes.len() / Self::ELEMENT_BYTES;

        if (p as usize) % mem::align_of::<u128>() != 0 {
            return Err(SerializationError::InvalidMemoryAlignment);
        }

        Ok(slice::from_raw_parts(p as *const Self, len))
    }

    fn zeroed_vector(n: usize) -> Vec<Self> {
        // this uses a specialized vector initialization code which requests zero-filled memory
        // from the OS; unfortunately, this works only for built-in types and we can't use
        // Self::ZERO here as much less efficient initialization procedure will be invoked.
        // We also use u128 to make sure the memory is aligned correctly for our element size.
        debug_assert_eq!(Self::ELEMENT_BYTES, mem::size_of::<u128>() * 2);
        let result = vec![0u128; n * 2];

        // translate a zero-filled vector of u128s into a vector of extension field elements
        let mut v = std::mem::ManuallyDrop::new(result);
        let p = v.as_mut_ptr();
        let len = v.len() / 2;
        let cap = v.capacity() / 2;
        unsafe { Vec::from_raw_parts(p as *mut Self, len, cap) }
    }

    fn prng_vector(seed: [u8; 32], n: usize) -> Vec<Self> {
        // get twice the number of base elements
        let result = BaseElement::prng_vector(seed, n * 2);

        // re-interpret vector of base elements as a vector of quad elements (but half the length)
        let mut v = std::mem::ManuallyDrop::new(result);
        let p = v.as_mut_ptr();
        let len = v.len() / 2;
        let cap = v.capacity() / 2;
        unsafe { Vec::from_raw_parts(p as *mut Self, len, cap) }
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

impl AddAssign for QuadElement {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

impl Sub for QuadElement {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl SubAssign for QuadElement {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
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

impl MulAssign for QuadElement {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs
    }
}

impl Div for QuadElement {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self {
        self * rhs.inv()
    }
}

impl DivAssign for QuadElement {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs
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
