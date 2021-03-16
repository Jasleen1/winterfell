use super::{FieldElement, FromVec, StarkField};
use core::convert::TryFrom;
use core::fmt::{Debug, Display, Formatter};
use core::ops::{Add, Div, Mul, Neg, Sub};
use serde::{Deserialize, Serialize};

// EXTENSION FIELD ELEMENT
// ================================================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct QuadExtension<E: StarkField>(E, E);

impl<E: StarkField> FieldElement for QuadExtension<E> {
    type PositiveInteger = E::PositiveInteger;
    type Base = E;

    const ELEMENT_BYTES: usize = E::ELEMENT_BYTES * 2;
    const ZERO: Self = Self(E::ZERO, E::ZERO);
    const ONE: Self = Self(E::ONE, E::ZERO);

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
        Self(self.0 + self.1, E::ZERO - self.1)
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

impl<E: StarkField> FromVec<E> for QuadExtension<E> {}

impl<E: StarkField> Display for QuadExtension<E> {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

// OVERLOADED OPERATORS
// ================================================================================================

impl<E: StarkField> Add for QuadExtension<E> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl<E: StarkField> Sub for QuadExtension<E> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl<E: StarkField> Mul for QuadExtension<E> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let coef0_mul = self.0 * rhs.0;
        Self(
            coef0_mul + self.1 * rhs.1,
            (self.0 + self.1) * (rhs.0 + rhs.1) - coef0_mul,
        )
    }
}

impl<E: StarkField> Div for QuadExtension<E> {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self {
        self * rhs.inv()
    }
}

impl<E: StarkField> Neg for QuadExtension<E> {
    type Output = Self;

    fn neg(self) -> Self {
        Self(E::ZERO - self.0, E::ZERO - self.1)
    }
}

// TYPE CONVERSIONS
// ================================================================================================

impl<E: StarkField> From<E> for QuadExtension<E> {
    fn from(e: E) -> Self {
        Self(e, E::ZERO)
    }
}

impl<E: StarkField> From<u128> for QuadExtension<E> {
    fn from(value: u128) -> Self {
        QuadExtension(E::from(value), E::ZERO)
    }
}

impl<E: StarkField> From<u64> for QuadExtension<E> {
    fn from(value: u64) -> Self {
        QuadExtension(E::from(value), E::ZERO)
    }
}

impl<E: StarkField> From<u32> for QuadExtension<E> {
    fn from(value: u32) -> Self {
        QuadExtension(E::from(value), E::ZERO)
    }
}

impl<E: StarkField> From<u16> for QuadExtension<E> {
    fn from(value: u16) -> Self {
        QuadExtension(E::from(value), E::ZERO)
    }
}

impl<E: StarkField> From<u8> for QuadExtension<E> {
    fn from(value: u8) -> Self {
        QuadExtension(E::from(value), E::ZERO)
    }
}

impl<'a, E: StarkField> TryFrom<&'a [u8]> for QuadExtension<E> {
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
