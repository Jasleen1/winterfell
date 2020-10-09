use super::traits::StarkField;
use crate::utils;
use core::{
    convert::{TryFrom, TryInto},
    fmt::{Debug, Display, Formatter},
    iter::{Product, Sum},
    ops::{Add, Div, Mul, Neg, Range, Sub},
};
use rand::{distributions::Uniform, prelude::*};

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

// Field modulus = 2^128 - 45 * 2^40 + 1
const M: u128 = 340282366920938463463374557953744961537;

// 2^40 root of unity
const G: u128 = 23953097886125630542083529559205016746;

const RANGE: Range<u128> = Range { start: 0, end: M };

// FIELD ELEMENT
// ================================================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct FieldElement(u128);

impl StarkField for FieldElement {
    type PositiveInteger = u128;

    /// sage: MODULUS = 2^128 - 45 * 2^40 + 1
    /// sage: GF(MODULUS).is_prime_field()
    /// True
    /// sage: GF(MODULUS).order()
    /// 340282366920938463463374557953744961537
    const MODULUS: Self::PositiveInteger = M;
    const MODULUS_BITS: u32 = 128;

    /// sage: GF(MODULUS).primitive_element()
    /// 3
    const GENERATOR: Self = FieldElement(3);

    /// sage: is_odd((MODULUS - 1) / 2^40)
    /// True
    const TWO_ADICITY: usize = 40;

    /// sage: k = (MODULUS - 1) / 2^40
    /// sage: GF(MODULUS).primitive_element()^k
    /// 23953097886125630542083529559205016746
    const TWO_ADIC_ROOT_OF_UNITY: Self = FieldElement(G);

    const ZERO: Self = FieldElement(0);
    const ONE: Self = FieldElement(1);

    fn inv(self) -> Self {
        FieldElement(inv(self.0))
    }

    fn exp(self, power: u128) -> Self {
        FieldElement(exp(self.0, power))
    }

    fn rand() -> Self {
        let range = Uniform::from(RANGE);
        let mut g = thread_rng();
        FieldElement(g.sample(range))
    }

    /// This implementation is about 5% faster than the one in the trait.
    fn get_power_series(b: Self, n: usize) -> Vec<Self> {
        let mut result = utils::uninit_vector(n);
        result[0] = FieldElement::ONE;
        for i in 1..result.len() {
            result[i] = result[i - 1] * b;
        }
        result
    }

    fn prng_vector(seed: [u8; 32], n: usize) -> Vec<Self> {
        let range = Uniform::from(RANGE);
        let g = StdRng::from_seed(seed);
        g.sample_iter(range).take(n).map(FieldElement).collect()
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }
}

impl Display for FieldElement {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// OVERLOADED OPERATORS
// ================================================================================================

impl Add for FieldElement {
    type Output = FieldElement;

    fn add(self, rhs: FieldElement) -> FieldElement {
        FieldElement(add(self.0, rhs.0))
    }
}

impl<'a> Add<&'a Self> for FieldElement {
    type Output = FieldElement;

    fn add(self, rhs: &'a FieldElement) -> Self::Output {
        FieldElement(add(self.0, rhs.0))
    }
}

impl Sub for FieldElement {
    type Output = FieldElement;

    fn sub(self, rhs: FieldElement) -> FieldElement {
        FieldElement(sub(self.0, rhs.0))
    }
}

impl<'a> Sub<&'a Self> for FieldElement {
    type Output = FieldElement;

    fn sub(self, rhs: &'a FieldElement) -> Self::Output {
        FieldElement(sub(self.0, rhs.0))
    }
}

impl Mul for FieldElement {
    type Output = FieldElement;

    fn mul(self, rhs: FieldElement) -> FieldElement {
        FieldElement(mul(self.0, rhs.0))
    }
}

impl<'a> Mul<&'a Self> for FieldElement {
    type Output = FieldElement;

    fn mul(self, rhs: &'a FieldElement) -> Self::Output {
        FieldElement(mul(self.0, rhs.0))
    }
}

impl Div for FieldElement {
    type Output = FieldElement;

    fn div(self, rhs: FieldElement) -> FieldElement {
        FieldElement(mul(self.0, inv(rhs.0)))
    }
}

impl<'a> Div<&'a Self> for FieldElement {
    type Output = FieldElement;

    fn div(self, rhs: &'a FieldElement) -> Self::Output {
        FieldElement(mul(self.0, inv(rhs.0)))
    }
}

impl Neg for FieldElement {
    type Output = FieldElement;

    fn neg(self) -> FieldElement {
        Self(sub(0, self.0))
    }
}

impl Sum for FieldElement {
    fn sum<I: Iterator<Item = FieldElement>>(iter: I) -> FieldElement {
        let mut result = 0;
        for value in iter {
            result = add(result, value.0)
        }
        FieldElement(result)
    }
}

impl<'a> Sum<&'a Self> for FieldElement {
    fn sum<I: Iterator<Item = &'a FieldElement>>(iter: I) -> FieldElement {
        let mut result = 0;
        for value in iter {
            result = add(result, value.0)
        }
        FieldElement(result)
    }
}

impl Product for FieldElement {
    fn product<I: Iterator<Item = FieldElement>>(mut iter: I) -> FieldElement {
        let mut result = match iter.next() {
            Some(value) => value.0,
            None => return FieldElement(0),
        };

        for value in iter {
            result = mul(result, value.0);
        }

        FieldElement(result)
    }
}

impl<'a> Product<&'a Self> for FieldElement {
    fn product<I: Iterator<Item = &'a FieldElement>>(mut iter: I) -> FieldElement {
        let mut result = match iter.next() {
            Some(value) => value.0,
            None => return FieldElement(0),
        };

        for value in iter {
            result = mul(result, value.0);
        }

        FieldElement(result)
    }
}

// TYPE CONVERSIONS
// ================================================================================================

impl From<u128> for FieldElement {
    /// Converts a 128-bit value into a filed element. If the value is greater than or equal to
    /// the field modulus, modular reduction is silently preformed.
    fn from(value: u128) -> Self {
        FieldElement(if value < M { value } else { value - M })
    }
}

impl From<u64> for FieldElement {
    /// Converts a 64-bit value into a filed element.
    fn from(value: u64) -> Self {
        FieldElement(value as u128)
    }
}

impl From<u32> for FieldElement {
    /// Converts a 32-bit value into a filed element.
    fn from(value: u32) -> Self {
        FieldElement(value as u128)
    }
}

impl From<u16> for FieldElement {
    /// Converts a 16-bit value into a filed element.
    fn from(value: u16) -> Self {
        FieldElement(value as u128)
    }
}

impl From<u8> for FieldElement {
    /// Converts an 8-bit value into a filed element.
    fn from(value: u8) -> Self {
        FieldElement(value as u128)
    }
}

impl From<[u8; 16]> for FieldElement {
    /// Converts the value encoded in an array of 16 bytes into a field element. The bytes
    /// are assumed to be in little-endian byte order. If the value is greater than or equal
    /// to the field modulus, modular reduction is silently preformed.
    fn from(bytes: [u8; 16]) -> Self {
        let value = u128::from_le_bytes(bytes);
        FieldElement::from(value)
    }
}

impl<'a> TryFrom<&'a [u8]> for FieldElement {
    type Error = String;

    /// Converts a slice of bytes into a field element; returns error if the value encoded in bytes
    /// is not a valid field element. The bytes are assumed to be in little-endian byte order.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let value = match bytes.try_into() {
            Ok(bytes) => u128::from_le_bytes(bytes),
            Err(error) => return Err(format!("{}", error)),
        };
        if value >= M {
            return Err(format!(
                "cannot convert bytes into a field element: \
                value {} is greater or equal to the field modulus",
                value
            ));
        }
        Ok(FieldElement(value))
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
    let (x0, x1, x2) = mul_128x64(a, (b >> 64) as u64); // x = a * b_hi
    let (mut x0, mut x1, x2) = mul_reduce(x0, x1, x2); // x = x - (x >> 128) * m
    if x2 == 1 {
        // if there was an overflow beyond 128 bits, subtract
        // modulus from the result to make sure it fits into
        // 128 bits; this can potentially be removed in favor
        // of checking overflow later
        let (t0, t1) = sub_modulus(x0, x1); // x = x - m
        x0 = t0;
        x1 = t1;
    }

    let (y0, y1, y2) = mul_128x64(a, b as u64); // y = a * b_lo

    let (mut y1, carry) = add64_with_carry(y1, x0, 0); // y = y + (x << 64)
    let (mut y2, y3) = add64_with_carry(y2, x1, carry);
    if y3 == 1 {
        // if there was an overflow beyond 192 bits, subtract
        // modulus * 2^64 from the result to make sure it fits
        // into 192 bits; this can potentially replace the
        // previous overflow check (but needs to be proven)
        let (t0, t1) = sub_modulus(y1, y2); // y = y - (m << 64)
        y1 = t0;
        y2 = t1;
    }

    let (mut z0, mut z1, z2) = mul_reduce(y0, y1, y2); // z = y - (y >> 128) * m

    // make sure z is smaller than m
    if z2 == 1 || (z1 == (M >> 64) as u64 && z0 >= (M as u64)) {
        let (t0, t1) = sub_modulus(z0, z1); // z = z - m
        z0 = t0;
        z1 = t1;
    }

    ((z1 as u128) << 64) + (z0 as u128)
}

/// Computes y such that (x * y) % m = 1 except for when when x = 0; in such a case,
/// 0 is returned; x is assumed to be a valid field element.
fn inv(x: u128) -> u128 {
    if x == 0 {
        return 0;
    };

    // initialize v, a, u, and d variables
    let mut v = M;
    let (mut a0, mut a1, mut a2) = (0, 0, 0);
    let (mut u0, mut u1, mut u2) = if x & 1 == 1 {
        // u = x
        (x as u64, (x >> 64) as u64, 0)
    } else {
        // u = x + m
        add_192x192(x as u64, (x >> 64) as u64, 0, M as u64, (M >> 64) as u64, 0)
    };
    // d = m - 1
    let (mut d0, mut d1, mut d2) = ((M as u64) - 1, (M >> 64) as u64, 0);

    // compute the inverse
    while v != 1 {
        while u2 > 0 || ((u0 as u128) + ((u1 as u128) << 64)) > v {
            // u > v
            // u = u - v
            let (t0, t1, t2) = sub_192x192(u0, u1, u2, v as u64, (v >> 64) as u64, 0);
            u0 = t0;
            u1 = t1;
            u2 = t2;

            // d = d + a
            let (t0, t1, t2) = add_192x192(d0, d1, d2, a0, a1, a2);
            d0 = t0;
            d1 = t1;
            d2 = t2;

            while u0 & 1 == 0 {
                if d0 & 1 == 1 {
                    // d = d + m
                    let (t0, t1, t2) = add_192x192(d0, d1, d2, M as u64, (M >> 64) as u64, 0);
                    d0 = t0;
                    d1 = t1;
                    d2 = t2;
                }

                // u = u >> 1
                u0 = (u0 >> 1) | ((u1 & 1) << 63);
                u1 = (u1 >> 1) | ((u2 & 1) << 63);
                u2 >>= 1;

                // d = d >> 1
                d0 = (d0 >> 1) | ((d1 & 1) << 63);
                d1 = (d1 >> 1) | ((d2 & 1) << 63);
                d2 >>= 1;
            }
        }

        // v = v - u (u is less than v at this point)
        v -= (u0 as u128) + ((u1 as u128) << 64);

        // a = a + d
        let (t0, t1, t2) = add_192x192(a0, a1, a2, d0, d1, d2);
        a0 = t0;
        a1 = t1;
        a2 = t2;

        while v & 1 == 0 {
            if a0 & 1 == 1 {
                // a = a + m
                let (t0, t1, t2) = add_192x192(a0, a1, a2, M as u64, (M >> 64) as u64, 0);
                a0 = t0;
                a1 = t1;
                a2 = t2;
            }

            v >>= 1;

            // a = a >> 1
            a0 = (a0 >> 1) | ((a1 & 1) << 63);
            a1 = (a1 >> 1) | ((a2 & 1) << 63);
            a2 >>= 1;
        }
    }

    // a = a mod m
    let mut a = (a0 as u128) + ((a1 as u128) << 64);
    while a2 > 0 || a >= M {
        let (t0, t1, t2) = sub_192x192(a0, a1, a2, M as u64, (M >> 64) as u64, 0);
        a0 = t0;
        a1 = t1;
        a2 = t2;
        a = (a0 as u128) + ((a1 as u128) << 64);
    }

    a
}

/// Computes (b^p) % m; b and p are assumed to be valid field elements.
pub fn exp(b: u128, p: u128) -> u128 {
    if b == 0 {
        return 0;
    } else if p == 0 {
        return 1;
    }

    let mut r = 1;
    let mut b = b;
    let mut p = p;

    // TODO: optimize
    while p > 0 {
        if p & 1 == 1 {
            r = mul(r, b);
        }
        p >>= 1;
        b = mul(b, b);
    }

    r
}

// HELPER FUNCTIONS
// ================================================================================================

#[inline]
fn mul_128x64(a: u128, b: u64) -> (u64, u64, u64) {
    let z_lo = ((a as u64) as u128) * (b as u128);
    let z_hi = (a >> 64) * (b as u128);
    let z_hi = z_hi + (z_lo >> 64);
    (z_lo as u64, z_hi as u64, (z_hi >> 64) as u64)
}

#[inline]
fn mul_reduce(z0: u64, z1: u64, z2: u64) -> (u64, u64, u64) {
    let (q0, q1, q2) = mul_by_modulus(z2);
    let (z0, z1, z2) = sub_192x192(z0, z1, z2, q0, q1, q2);
    (z0, z1, z2)
}

#[inline]
fn mul_by_modulus(a: u64) -> (u64, u64, u64) {
    let a_lo = (a as u128).wrapping_mul(M);
    let a_hi = if a == 0 { 0 } else { a - 1 };
    (a_lo as u64, (a_lo >> 64) as u64, a_hi)
}

#[inline]
fn sub_modulus(a_lo: u64, a_hi: u64) -> (u64, u64) {
    let mut z = 0u128.wrapping_sub(M);
    z = z.wrapping_add(a_lo as u128);
    z = z.wrapping_add((a_hi as u128) << 64);
    (z as u64, (z >> 64) as u64)
}

#[inline]
fn sub_192x192(a0: u64, a1: u64, a2: u64, b0: u64, b1: u64, b2: u64) -> (u64, u64, u64) {
    let z0 = (a0 as u128).wrapping_sub(b0 as u128);
    let z1 = (a1 as u128).wrapping_sub((b1 as u128) + (z0 >> 127));
    let z2 = (a2 as u128).wrapping_sub((b2 as u128) + (z1 >> 127));
    (z0 as u64, z1 as u64, z2 as u64)
}

#[inline]
fn add_192x192(a0: u64, a1: u64, a2: u64, b0: u64, b1: u64, b2: u64) -> (u64, u64, u64) {
    let z0 = (a0 as u128) + (b0 as u128);
    let z1 = (a1 as u128) + (b1 as u128) + (z0 >> 64);
    let z2 = (a2 as u128) + (b2 as u128) + (z1 >> 64);
    (z0 as u64, z1 as u64, z2 as u64)
}

#[inline]
pub const fn add64_with_carry(a: u64, b: u64, carry: u64) -> (u64, u64) {
    let ret = (a as u128) + (b as u128) + (carry as u128);
    (ret as u64, (ret >> 64) as u64)
}
