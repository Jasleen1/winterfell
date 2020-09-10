use std::mem;
use crate::field;
use crate::utils::{ uninit_vector, filled_vector };

#[cfg(test)]
mod tests;

// POLYNOMIAL EVALUATION
// ================================================================================================

/// Evaluates polynomial `p` at coordinate `x`
pub fn eval(p: &[u128], x: u128) -> u128 {
    let mut y = field::ZERO;
    let mut power_of_x = field::ONE;
    for i in 0..p.len() {
        y = field::add(y, field::mul(p[i], power_of_x));
        power_of_x = field::mul(power_of_x, x);
    }
    return y;
}

// POLYNOMIAL INTERPOLATION
// ================================================================================================

/// Uses Lagrange interpolation to build a polynomial from X and Y coordinates.
pub fn interpolate(xs: &[u128], ys: &[u128]) -> Vec<u128> {
    debug_assert!(xs.len() == ys.len(), "Number of X and Y coordinates must be the same");

    let roots = get_zero_roots(xs);
    let mut divisor = [field::ZERO, field::ONE];
    let mut numerators: Vec<Vec<u128>> = Vec::with_capacity(xs.len());
    for i in 0..xs.len() {
        divisor[0] = field::neg(xs[i]);
        numerators.push(div(&roots, &divisor));
    }

    let mut denominators: Vec<u128> = Vec::with_capacity(xs.len());
    for i in 0..xs.len() {
        denominators.push(eval(&numerators[i], xs[i]));
    }
    let denominators = field::inv_many(&denominators);

    let mut result = vec![field::ZERO; xs.len()];
    for i in 0..xs.len() {
        let y_slice = field::mul(ys[i], denominators[i]);
        for j in 0..xs.len() {
            if numerators[i][j] != field::ZERO && ys[i] != field::ZERO {
                result[j] = field::add(result[j], field::mul(numerators[i][j], y_slice));
            }
        }
    }

    return result;
}

// POLYNOMIAL MATH OPERATIONS
// ================================================================================================

/// Adds polynomial `a` to polynomial `b`
pub fn add(a: &[u128], b: &[u128]) -> Vec<u128> {
    let result_len = std::cmp::max(a.len(), b.len());
    let mut result = Vec::with_capacity(result_len);
    for i in 0..result_len {
        let c1 = if i < a.len() { a[i] } else { field::ZERO };
        let c2 = if i < b.len() { b[i] } else { field::ZERO };
        result.push(field::add(c1, c2));
    }
    return result;
}

/// Subtracts polynomial `b` from polynomial `a`
pub fn sub(a: &[u128], b: &[u128]) -> Vec<u128> {
    let result_len = std::cmp::max(a.len(), b.len());
    let mut result = Vec::with_capacity(result_len);
    for i in 0..result_len {
        let c1 = if i < a.len() { a[i] } else { field::ZERO };
        let c2 = if i < b.len() { b[i] } else { field::ZERO };
        result.push(field::sub(c1, c2));
    }
    return result;
}

/// Multiplies polynomial `a` by polynomial `b`
pub fn mul(a: &[u128], b: &[u128]) -> Vec<u128> {
    let result_len = a.len() + b.len() - 1;
    let mut result = vec![field::ZERO; result_len];
    for i in 0..a.len() {
        for j in 0..b.len() {
            let s = field::mul(a[i], b[j]);
            result[i + j] = field::add(result[i + j], s);
        }
    }
    return result;
}

/// Multiplies every coefficient of polynomial `p` by constant `k`
pub fn mul_by_const(p: &[u128], k: u128) -> Vec<u128> {
    let mut result = Vec::with_capacity(p.len());
    for i in 0..p.len() {
        result.push(field::mul(p[i], k));
    }
    return result;
}

/// Divides polynomial `a` by polynomial `b`; if the polynomials don't divide evenly,
/// the remainder is ignored.
pub fn div(a: &[u128], b: &[u128]) -> Vec<u128> {

    let mut apos = degree_of(a);
    let mut a = a.to_vec();

    let bpos = degree_of(b);
    assert!(apos >= bpos, "cannot divide by polynomial of higher degree");
    if bpos == 0 {
        assert!(b[0] != field::ZERO, "cannot divide polynomial by zero");
    }

    let mut result = vec![field::ZERO; apos - bpos + 1];
    for i in (0..result.len()).rev() {
        let quot = field::div(a[apos], b[bpos]);
        result[i] = quot;
        for j in (0..bpos).rev() {
            a[i + j] = field::sub(a[i + j], field::mul(b[j], quot));
        }
        apos = apos.wrapping_sub(1);
    }

    return result;
}

/// Divides polynomial `a` by binomial (x - `b`) using Synthetic division method;
/// if the polynomials don't divide evenly, the remainder is ignored.
pub fn syn_div(a: &[u128], b: u128) -> Vec<u128> {
    let mut result = a.to_vec();
    syn_div_in_place(&mut result, b);
    return result;
}

/// Divides polynomial `a` by binomial (x - `b`) using Synthetic division method and stores the
/// result in `a`; if the polynomials don't divide evenly, the remainder is ignored.
pub fn syn_div_in_place(a: &mut [u128], b: u128) {
    let mut c = field::ZERO;
    for i in (0..a.len()).rev() {
        let temp = field::add(a[i], field::mul(b, c));
        a[i] = c;
        c = temp;
    }
}

/// Divides polynomial `a` by polynomial (x^degree - 1) / (x - exceptions[i]) for all i using
/// Synthetic division method and stores the result in `a`; if the polynomials don't divide evenly,
/// the remainder is ignored.
pub fn syn_div_expanded_in_place(a: &mut [u128], degree: usize, exceptions: &[u128]) {

    // allocate space for the result
    let mut result = filled_vector(a.len(), a.len() + exceptions.len(), field::ZERO);

    // compute a / (x^degree - 1)
    result.copy_from_slice(&a);
    let degree_offset = a.len() - degree;
    for i in (0..degree_offset).rev() {
        result[i] = field::add(result[i], result[i + degree]);
    }

    // multiply result by (x - exceptions[i]) in place
    for &exception in exceptions {

        // exception term is negative
        let exception = field::neg(exception);

        // extend length of result since we are raising degree
        unsafe { result.set_len(result.len() + 1); }

        let mut next_term = result[0];
        result[0] = field::ZERO;
        for i in 0..(result.len() - 1) {
            result[i] = field::add(result[i], field::mul(next_term, exception));
            mem::swap(&mut next_term, &mut result[i + 1]);
        }
    }

    // copy result back into `a` skipping remainder terms
    a[..(degree_offset + exceptions.len())].copy_from_slice(&result[degree..]);

    // fill the rest of the result with 0
    for i in (degree_offset + exceptions.len())..a.len() { a[i] = field::ZERO; }
}

// DEGREE INFERENCE
// ================================================================================================

/// Returns degree of the polynomial `poly`
pub fn degree_of(poly: &[u128]) -> usize {
    for i in (0..poly.len()).rev() {
        if poly[i] != field::ZERO { return i; }
    }
    return 0;
}

// HELPER FUNCTIONS
// ================================================================================================
fn get_zero_roots(xs: &[u128]) -> Vec<u128> {
    let mut n = xs.len() + 1;
    let mut result = uninit_vector(n);

    n -= 1;
    result[n] = field::ONE;

    for i in 0..xs.len() {
        n -= 1;
        result[n] = field::ZERO;
        for j in n..xs.len() {
            result[j] = field::sub(result[j], field::mul(result[j + 1], xs[i]));
        }
    }

    return result;
}
