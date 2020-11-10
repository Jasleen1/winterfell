use crate::field::FieldElementTrait;
use crate::utils;
use std::mem;

#[cfg(test)]
mod tests;

// POLYNOMIAL EVALUATION
// ================================================================================================

/// Evaluates polynomial `p` at coordinate `x`.
pub fn eval<E: FieldElementTrait>(p: &[E], x: E) -> E {
    // Horner evaluation
    p.iter().rev().fold(E::ZERO, |acc, coeff| acc * x + *coeff)
}

/// Evaluates polynomial `p` at all coordinates in `xs` slice.
pub fn eval_many<E: FieldElementTrait>(p: &[E], xs: &[E]) -> Vec<E> {
    xs.iter().map(|x| eval(p, *x)).collect()
}

// POLYNOMIAL INTERPOLATION
// ================================================================================================

/// Uses Lagrange interpolation to build a polynomial from X and Y coordinates.
pub fn interpolate<E: FieldElementTrait>(xs: &[E], ys: &[E], remove_leading_zeros: bool) -> Vec<E> {
    debug_assert!(
        xs.len() == ys.len(),
        "Number of X and Y coordinates must be the same"
    );

    let roots = get_zero_roots(xs);
    let mut divisor = [E::ZERO, E::ONE];
    let mut numerators: Vec<Vec<E>> = Vec::with_capacity(xs.len());
    for xcoord in xs {
        divisor[0] = -*xcoord;
        numerators.push(div(&roots, &divisor));
    }

    let mut denominators: Vec<E> = Vec::with_capacity(xs.len());
    for i in 0..xs.len() {
        denominators.push(eval(&numerators[i], xs[i]));
    }
    let denominators = E::inv_many(&denominators);

    let mut result = vec![E::ZERO; xs.len()];
    for i in 0..xs.len() {
        let y_slice = ys[i] * denominators[i];
        if ys[i] != E::ZERO {
            for (j, res) in result.iter_mut().enumerate() {
                if numerators[i][j] != E::ZERO {
                    *res = *res + numerators[i][j] * y_slice;
                }
            }
        }
    }

    if remove_leading_zeros {
        utils::remove_leading_zeros(&result)
    } else {
        result
    }
}

// POLYNOMIAL MATH OPERATIONS
// ================================================================================================

/// Adds polynomial `a` to polynomial `b`
pub fn add<E: FieldElementTrait>(a: &[E], b: &[E]) -> Vec<E> {
    let result_len = std::cmp::max(a.len(), b.len());
    let mut result = Vec::with_capacity(result_len);
    for i in 0..result_len {
        let c1 = if i < a.len() { a[i] } else { E::ZERO };
        let c2 = if i < b.len() { b[i] } else { E::ZERO };
        result.push(c1 + c2);
    }
    result
}

/// Subtracts polynomial `b` from polynomial `a`
pub fn sub<E: FieldElementTrait>(a: &[E], b: &[E]) -> Vec<E> {
    let result_len = std::cmp::max(a.len(), b.len());
    let mut result = Vec::with_capacity(result_len);
    for i in 0..result_len {
        let c1 = if i < a.len() { a[i] } else { E::ZERO };
        let c2 = if i < b.len() { b[i] } else { E::ZERO };
        result.push(c1 - c2);
    }
    result
}

/// Multiplies polynomial `a` by polynomial `b`
pub fn mul<E: FieldElementTrait>(a: &[E], b: &[E]) -> Vec<E> {
    let result_len = a.len() + b.len() - 1;
    let mut result = vec![E::ZERO; result_len];
    for i in 0..a.len() {
        for j in 0..b.len() {
            let s = a[i] * b[j];
            result[i + j] = result[i + j] + s;
        }
    }
    result
}

/// Multiplies every coefficient of polynomial `p` by constant `k`
pub fn mul_by_const<E: FieldElementTrait>(p: &[E], k: E) -> Vec<E> {
    let mut result = Vec::with_capacity(p.len());
    for coeff in p {
        result.push(*coeff * k);
    }
    result
}

/// Divides polynomial `a` by polynomial `b`; if the polynomials don't divide evenly,
/// the remainder is ignored.
pub fn div<E: FieldElementTrait>(a: &[E], b: &[E]) -> Vec<E> {
    let mut apos = degree_of(a);
    let mut a = a.to_vec();

    let bpos = degree_of(b);
    assert!(apos >= bpos, "cannot divide by polynomial of higher degree");
    if bpos == 0 {
        assert!(b[0] != E::ZERO, "cannot divide polynomial by zero");
    }

    let mut result = vec![E::ZERO; apos - bpos + 1];
    for i in (0..result.len()).rev() {
        let quot = a[apos] / b[bpos];
        result[i] = quot;
        for j in (0..bpos).rev() {
            a[i + j] = a[i + j] - b[j] * quot;
        }
        apos = apos.wrapping_sub(1);
    }

    result
}

/// Divides polynomial `a` by binomial (x - `b`) using Synthetic division method;
/// if the polynomials don't divide evenly, the remainder is ignored.
pub fn syn_div<E: FieldElementTrait>(a: &[E], b: E) -> Vec<E> {
    let mut result = a.to_vec();
    syn_div_in_place(&mut result, b);
    result
}

/// Divides polynomial `a` by binomial (x - `b`) using Synthetic division method and stores the
/// result in `a`; if the polynomials don't divide evenly, the remainder is ignored.
pub fn syn_div_in_place<E: FieldElementTrait>(a: &mut [E], b: E) {
    let mut c = E::ZERO;
    for i in (0..a.len()).rev() {
        let temp = a[i] + b * c;
        a[i] = c;
        c = temp;
    }
}

/// Divides polynomial `a` by polynomial (x^degree - 1) / (x - exceptions[i]) for all i using
/// Synthetic division method and stores the result in `a`; if the polynomials don't divide evenly,
/// the remainder is ignored.
pub fn syn_div_expanded_in_place<E: FieldElementTrait>(
    a: &mut [E],
    degree: usize,
    exceptions: &[E],
) {
    // allocate space for the result
    let mut result = utils::filled_vector(a.len(), a.len() + exceptions.len(), E::ZERO);

    // compute a / (x^degree - 1)
    result.copy_from_slice(&a);
    let degree_offset = a.len() - degree;
    for i in (0..degree_offset).rev() {
        result[i] = result[i] + result[i + degree];
    }

    // multiply result by (x - exceptions[i]) in place
    for &exception in exceptions {
        // exception term is negative
        let exception = -exception;

        // extend length of result since we are raising degree
        unsafe {
            result.set_len(result.len() + 1);
        }

        let mut next_term = result[0];
        result[0] = E::ZERO;
        for i in 0..(result.len() - 1) {
            result[i] = result[i] + next_term * exception;
            mem::swap(&mut next_term, &mut result[i + 1]);
        }
    }

    // copy result back into `a` skipping remainder terms
    a[..(degree_offset + exceptions.len())].copy_from_slice(&result[degree..]);

    // fill the rest of the result with 0
    for res in a.iter_mut().skip(degree_offset + exceptions.len()) {
        *res = E::ZERO;
    }
}

// DEGREE INFERENCE
// ================================================================================================

/// Returns degree of the polynomial `poly`
pub fn degree_of<E: FieldElementTrait>(poly: &[E]) -> usize {
    for i in (0..poly.len()).rev() {
        if poly[i] != E::ZERO {
            return i;
        }
    }
    0
}

// HELPER FUNCTIONS
// ================================================================================================
fn get_zero_roots<E: FieldElementTrait>(xs: &[E]) -> Vec<E> {
    let mut n = xs.len() + 1;
    let mut result = utils::uninit_vector(n);

    n -= 1;
    result[n] = E::ONE;

    for i in 0..xs.len() {
        n -= 1;
        result[n] = E::ZERO;
        for j in n..xs.len() {
            result[j] = result[j] - result[j + 1] * xs[i];
        }
    }

    result
}
