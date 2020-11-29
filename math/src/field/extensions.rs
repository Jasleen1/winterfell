use super::{FieldElement, FromVec};
use core::convert::TryFrom;
use core::fmt::{Debug, Display, Formatter};
use core::ops::{Add, Div, Mul, Neg, Sub};
use serde::{Deserialize, Serialize};

// EXTENSION FIELD ELEMENT
// ================================================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExtensionElement<E: FieldElement>(E, E);

impl<E: FieldElement> ExtensionElement<E> {
    pub fn conjugate(&self) -> Self {
        Self(self.0 + self.1, E::ZERO - self.1)
    }
}

impl<E: FieldElement> FieldElement for ExtensionElement<E> {
    type PositiveInteger = E::PositiveInteger;

    const ELEMENT_BYTES: usize = E::ELEMENT_BYTES * 2;
    const ZERO: Self = Self(E::ZERO, E::ZERO);
    const ONE: Self = Self(E::ONE, E::ZERO);

    fn inv(self) -> Self {
        if self == Self::ZERO {
            return Self::ZERO;
        }
        let denom = self.0 * self.0 + self.0 * self.1 - self.1 * self.1;
        let denom_inv = denom.inv();
        Self((self.0 + self.1) * denom_inv, self.1.neg() * denom_inv)
    }

    fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
        Self::try_from(&bytes[..Self::ELEMENT_BYTES as usize]).ok()
    }

    fn rand() -> Self {
        Self(E::rand(), E::rand())
    }

    fn to_bytes(&self) -> Vec<u8> {
        [self.0.to_bytes(), self.1.to_bytes()].concat()
    }
}

impl<E: FieldElement> FromVec<E> for ExtensionElement<E> {}

impl<E: FieldElement> Display for ExtensionElement<E> {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

// OVERLOADED OPERATORS
// ================================================================================================

impl<E: FieldElement> Add for ExtensionElement<E> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl<E: FieldElement> Sub for ExtensionElement<E> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl<E: FieldElement> Mul for ExtensionElement<E> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let coef0_mul = self.0 * rhs.0;
        Self(
            coef0_mul + self.1 * rhs.1,
            (self.0 + self.1) * (rhs.0 + rhs.1) - coef0_mul,
        )
    }
}

impl<E: FieldElement> Div for ExtensionElement<E> {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self {
        self * rhs.inv()
    }
}

impl<E: FieldElement> Neg for ExtensionElement<E> {
    type Output = Self;

    fn neg(self) -> Self {
        Self(E::ZERO - self.0, E::ZERO - self.1)
    }
}

// TYPE CONVERSIONS
// ================================================================================================

impl<E: FieldElement> From<E> for ExtensionElement<E> {
    fn from(e: E) -> Self {
        Self(e, E::ZERO)
    }
}

impl<E: FieldElement> From<u128> for ExtensionElement<E> {
    fn from(value: u128) -> Self {
        ExtensionElement(E::from(value), E::ZERO)
    }
}

impl<E: FieldElement> From<u64> for ExtensionElement<E> {
    fn from(value: u64) -> Self {
        ExtensionElement(E::from(value), E::ZERO)
    }
}

impl<E: FieldElement> From<u32> for ExtensionElement<E> {
    fn from(value: u32) -> Self {
        ExtensionElement(E::from(value), E::ZERO)
    }
}

impl<E: FieldElement> From<u16> for ExtensionElement<E> {
    fn from(value: u16) -> Self {
        ExtensionElement(E::from(value), E::ZERO)
    }
}

impl<E: FieldElement> From<u8> for ExtensionElement<E> {
    fn from(value: u8) -> Self {
        ExtensionElement(E::from(value), E::ZERO)
    }
}

impl<'a, E: FieldElement> TryFrom<&'a [u8]> for ExtensionElement<E> {
    type Error = String;

    /// Converts a slice of bytes into a field element; returns error if the value encoded in bytes
    /// is not a valid field element. The bytes are assumed to be in little-endian byte order.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < Self::ELEMENT_BYTES {
            return Err(
                "need more bytes in order to convert into extension field element".to_string(),
            );
        }
        let value0 = match E::try_from(&bytes[..E::ELEMENT_BYTES]) {
            Ok(val) => val,
            Err(_) => {
                return Err("could not convert into field element".to_string());
            }
        };
        let value1 = match E::try_from(&bytes[E::ELEMENT_BYTES..]) {
            Ok(val) => val,
            Err(_) => {
                return Err("could not convert into field element".to_string());
            }
        };
        Ok(Self(value0, value1))
    }
}
