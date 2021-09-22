use math::{
    fft,
    FieldElement, StarkField,
};
use std::convert::TryInto;
use crate::matrix_utils::*;
// TODO: Add error checking and throwing
/**
 * This is equivalent to computing v_H(X) for a multiplicative coset
 * H = eta * H_0 for a multiplicative subgroup H_0 of order dom_size
 * Note that v_H(X) = X^dom_size - eta^dom_size. If eta = 1 we're
 * in a multiplicative subgroup itself.
 **/
pub fn compute_vanishing_poly<E: FieldElement>(x: E, eta: E, dom_size: u128) -> E {
    let power_u64: u64 = dom_size.try_into().unwrap();
    let power = E::PositiveInteger::from(power_u64);
    if eta == E::ONE {
        x.exp(power) - eta
    } else {
        x.exp(power) - eta.exp(power)
    }
}

/**
 * Compute vanishing polynomial for a multiplicative subgroup. Same as above with
 * eta = ONE.
 **/
pub fn vanishing_poly_for_mult_subgroup<E: FieldElement>(x: E, dom_size: u128) -> E {
    compute_vanishing_poly(x, E::ONE, dom_size)
}

// This is equivalent to computing u_H(X, X) for a multiplicative coset H
// of order dom_size = |H|.
pub fn compute_derivative_on_single_val<E: FieldElement>(x: E, dom_size: u128) -> E {
    let dom_size_coeff = E::from(dom_size);
    let power_u64: u64 = (dom_size - 1).try_into().unwrap();
    let power = E::PositiveInteger::from(power_u64);
    dom_size_coeff * x.exp(power)
}

// Represents a binomial, i.e. a polynomial in two variables X and Y.
// The (i, j)th element of this binomial is the coefficient of X^i * Y^j
pub type BivariatePoly<E: FieldElement> = Vec<Vec<E>>;


pub fn compute_binomial_on_x<E: FieldElement>(bivariate: BivariatePoly<E>, x_val: E) -> Vec<E> {
    // Given a BivariatePoly, computes a monomial in Y, obtained by evaluating bivariate(x_val, Y)
    // represented by the output vector
    let bivariate_as_matrix = Matrix::new("binomial", bivariate).unwrap();
    let transposed_bivariate = bivariate_as_matrix.get_transpose("transposed_binomial");
    compute_binomial_on_y(transposed_bivariate.mat, x_val)
}

pub fn compute_binomial_on_y<E: FieldElement>(bivariate: BivariatePoly<E>, y_val: E) -> Vec<E> {
    // Given a BivariatePoly, computes a monomial in X, obtained by evaluating bivariate(X, y_val)
    // represented by the output vector
    // Note that since bivariate[i][j] is the coefficient of X^i Y^j, technically,
    // bivariate[i] * (Y^j)_j is the coeffient of X^i
    // Hence, evaluating the polynomial bivariate[i] on y_val gives the coeff of X^i
    let mut x_coeffs = Vec::new();
    for i in 0..bivariate.len() {
        x_coeffs.push(math::polynom::eval(&bivariate[i], y_val));
    }
    x_coeffs
}
