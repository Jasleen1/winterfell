use crate::field::{f128::FieldElement, StarkField};
use crate::utils;
use std::mem;

#[cfg(test)]
mod tests;

// POLYNOMIAL EVALUATION
// ================================================================================================

/// Evaluates polynomial `p` at coordinate `x`.
pub fn eval(p: &[FieldElement], x: FieldElement) -> FieldElement {
    // Horner evaluation
    p.iter()
        .rev()
        .fold(FieldElement::ZERO, |acc, coeff| acc * x + *coeff)
}

/// Evaluates polynomial `p` at all coordinates in `xs` slice.
pub fn eval_many(p: &[FieldElement], xs: &[FieldElement]) -> Vec<FieldElement> {
    xs.iter().map(|x| eval(p, *x)).collect()
}

// POLYNOMIAL INTERPOLATION
// ================================================================================================

/// Uses Lagrange interpolation to build a polynomial from X and Y coordinates.
pub fn interpolate(
    xs: &[FieldElement],
    ys: &[FieldElement],
    remove_leading_zeros: bool,
) -> Vec<FieldElement> {
    debug_assert!(
        xs.len() == ys.len(),
        "Number of X and Y coordinates must be the same"
    );

    let roots = get_zero_roots(xs);
    let mut divisor = [FieldElement::ZERO, FieldElement::ONE];
    let mut numerators: Vec<Vec<FieldElement>> = Vec::with_capacity(xs.len());
    for xcoord in xs {
        divisor[0] = -*xcoord;
        numerators.push(div(&roots, &divisor));
    }

    let mut denominators: Vec<FieldElement> = Vec::with_capacity(xs.len());
    for i in 0..xs.len() {
        denominators.push(eval(&numerators[i], xs[i]));
    }
    let denominators = FieldElement::inv_many(&denominators);

    let mut result = vec![FieldElement::ZERO; xs.len()];
    for i in 0..xs.len() {
        let y_slice = ys[i] * denominators[i];
        if ys[i] != FieldElement::ZERO {
            for (j, res) in result.iter_mut().enumerate() {
                if numerators[i][j] != FieldElement::ZERO {
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
pub fn add(a: &[FieldElement], b: &[FieldElement]) -> Vec<FieldElement> {
    let result_len = std::cmp::max(a.len(), b.len());
    let mut result = Vec::with_capacity(result_len);
    for i in 0..result_len {
        let c1 = if i < a.len() {
            a[i]
        } else {
            FieldElement::ZERO
        };
        let c2 = if i < b.len() {
            b[i]
        } else {
            FieldElement::ZERO
        };
        result.push(c1 + c2);
    }
    result
}

/// Subtracts polynomial `b` from polynomial `a`
pub fn sub(a: &[FieldElement], b: &[FieldElement]) -> Vec<FieldElement> {
    let result_len = std::cmp::max(a.len(), b.len());
    let mut result = Vec::with_capacity(result_len);
    for i in 0..result_len {
        let c1 = if i < a.len() {
            a[i]
        } else {
            FieldElement::ZERO
        };
        let c2 = if i < b.len() {
            b[i]
        } else {
            FieldElement::ZERO
        };
        result.push(c1 - c2);
    }
    result
}

/// Multiplies polynomial `a` by polynomial `b`
pub fn mul(a: &[FieldElement], b: &[FieldElement]) -> Vec<FieldElement> {
    let result_len = a.len() + b.len() - 1;
    let mut result = vec![FieldElement::ZERO; result_len];
    for i in 0..a.len() {
        for j in 0..b.len() {
            let s = a[i] * b[j];
            result[i + j] = result[i + j] + s;
        }
    }
    result
}

/// Multiplies every coefficient of polynomial `p` by constant `k`
pub fn mul_by_const(p: &[FieldElement], k: FieldElement) -> Vec<FieldElement> {
    let mut result = Vec::with_capacity(p.len());
    for coeff in p {
        result.push(*coeff * k);
    }
    result
}

/// Divides polynomial `a` by polynomial `b`; if the polynomials don't divide evenly,
/// the remainder is ignored.
pub fn div(a: &[FieldElement], b: &[FieldElement]) -> Vec<FieldElement> {
    let mut apos = degree_of(a);
    let mut a = a.to_vec();

    let bpos = degree_of(b);
    assert!(apos >= bpos, "cannot divide by polynomial of higher degree");
    if bpos == 0 {
        assert!(
            b[0] != FieldElement::ZERO,
            "cannot divide polynomial by zero"
        );
    }

    let mut result = vec![FieldElement::ZERO; apos - bpos + 1];
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
pub fn syn_div(a: &[FieldElement], b: FieldElement) -> Vec<FieldElement> {
    let mut result = a.to_vec();
    syn_div_in_place(&mut result, b);
    result
}

/// Divides polynomial `a` by binomial (x - `b`) using Synthetic division method and stores the
/// result in `a`; if the polynomials don't divide evenly, the remainder is ignored.
pub fn syn_div_in_place(a: &mut [FieldElement], b: FieldElement) {
    let mut c = FieldElement::ZERO;
    for i in (0..a.len()).rev() {
        let temp = a[i] + b * c;
        a[i] = c;
        c = temp;
    }
}

/// Divides polynomial `a` by polynomial (x^degree - 1) / (x - exceptions[i]) for all i using
/// Synthetic division method and stores the result in `a`; if the polynomials don't divide evenly,
/// the remainder is ignored.
pub fn syn_div_expanded_in_place(
    a: &mut [FieldElement],
    degree: usize,
    exceptions: &[FieldElement],
) {
    // allocate space for the result
    let mut result = utils::filled_vector(a.len(), a.len() + exceptions.len(), FieldElement::ZERO);

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
        result[0] = FieldElement::ZERO;
        for i in 0..(result.len() - 1) {
            result[i] = result[i] + next_term * exception;
            mem::swap(&mut next_term, &mut result[i + 1]);
        }
    }

    // copy result back into `a` skipping remainder terms
    a[..(degree_offset + exceptions.len())].copy_from_slice(&result[degree..]);

    // fill the rest of the result with 0
    for res in a.iter_mut().skip(degree_offset + exceptions.len()) {
        *res = FieldElement::ZERO;
    }
}

// DEGREE INFERENCE
// ================================================================================================

/// Returns degree of the polynomial `poly`
pub fn degree_of(poly: &[FieldElement]) -> usize {
    for i in (0..poly.len()).rev() {
        if poly[i] != FieldElement::ZERO {
            return i;
        }
    }
    0
}

// HELPER FUNCTIONS
// ================================================================================================
fn get_zero_roots(xs: &[FieldElement]) -> Vec<FieldElement> {
    let mut n = xs.len() + 1;
    let mut result = utils::uninit_vector(n);

    n -= 1;
    result[n] = FieldElement::ONE;

    for i in 0..xs.len() {
        n -= 1;
        result[n] = FieldElement::ZERO;
        for j in n..xs.len() {
            result[j] = result[j] - result[j + 1] * xs[i];
        }
    }

    result
}
