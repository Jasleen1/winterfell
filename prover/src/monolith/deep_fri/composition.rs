use super::super::types::{ConstraintPoly, PolyTable};
use super::super::utils;
use crate::proof::DeepValues;
use math::{field, polynom};

// COMPOSITION COEFFICIENTS
// ================================================================================================

pub struct CompositionCoefficients {
    pub trace1: Vec<u128>,
    pub trace2: Vec<u128>,
    pub t1_degree: u128,
    pub t2_degree: u128,
    pub constraints: u128,
}

impl CompositionCoefficients {
    pub fn new<T: Iterator<Item = u128>>(prng: &mut T, trace_width: usize) -> Self {
        CompositionCoefficients {
            trace1: prng.take(2 * trace_width).collect(),
            trace2: prng.take(2 * trace_width).collect(),
            t1_degree: prng.next().unwrap(),
            t2_degree: prng.next().unwrap(),
            constraints: prng.next().unwrap(),
        }
    }
}

// PROCEDURES
// ================================================================================================

pub fn draw_z_and_coefficients(
    seed: [u8; 32],
    trace_width: usize,
) -> (u128, CompositionCoefficients) {
    let mut prng = field::prng_iter(seed);
    let z = prng.next().unwrap();
    let coefficients = CompositionCoefficients::new(&mut prng, trace_width);
    (z, coefficients)
}

/// Combines trace polynomials for all registers into a single composition polynomial.
/// The combination is done as follows:
/// 1. First, state of trace registers at deep points z and z * g are computed;
/// 2. Then, polynomials T1_i(x) = (T_i(x) - T_i(z)) / (x - z) and
/// T2_i(x) = (T_i(x) - T_i(z * g)) / (x - z * g) are computed for all i and combined
/// together into a single polynomial using a pseudo-random linear combination;
/// 3. Then the degree of the polynomial is adjusted to match the composition degree
pub fn compose_trace_polys(
    composition_poly: &mut Vec<u128>,
    composition_degree: usize,
    polys: PolyTable,
    z: u128,
    cc: &CompositionCoefficients,
) -> DeepValues {
    let trace_length = polys.poly_size();

    let g = field::get_root_of_unity(trace_length);
    let next_z = field::mul(z, g);

    // compute state of registers at deep points z and z * g
    let trace_state1 = polys.evaluate_at(z);
    let trace_state2 = polys.evaluate_at(next_z);

    let mut t1_composition = vec![field::ZERO; trace_length];
    let mut t2_composition = vec![field::ZERO; trace_length];

    // combine trace polynomials into 2 composition polynomials T1(x) and T2(x)
    let polys = polys.into_vec();
    for i in 0..polys.len() {
        // compute T1(x) = (T(x) - T(z)), multiply it by a pseudo-random
        // coefficient, and add the result into composition polynomial
        acc_poly(
            &mut t1_composition,
            &polys[i],
            trace_state1[i],
            cc.trace1[i],
        );

        // compute T2(x) = (T(x) - T(z * g)), multiply it by a pseudo-random
        // coefficient, and add the result into composition polynomial
        acc_poly(
            &mut t2_composition,
            &polys[i],
            trace_state2[i],
            cc.trace2[i],
        );
    }

    // divide the two composition polynomials by (x - z) and (x - z * g)
    // respectively and add the resulting polynomials together
    polynom::syn_div_in_place(&mut t1_composition, z);
    polynom::syn_div_in_place(&mut t2_composition, next_z);
    utils::add_in_place(&mut t1_composition, &t2_composition);

    // adjust the degree of the polynomial to match the degree parameter by computing
    // C(x) = T(x) * k_1 + T(x) * x^incremental_degree * k_2
    let incremental_degree = get_incremental_trace_degree(composition_degree, trace_length);

    // this is equivalent to T(x) * k_1
    utils::mul_acc(
        &mut composition_poly[..trace_length],
        &t1_composition,
        cc.t1_degree,
    );
    // this is equivalent to T(x) * x^incremental_degree * k_2
    utils::mul_acc(
        &mut composition_poly[incremental_degree..(incremental_degree + trace_length)],
        &t1_composition,
        cc.t2_degree,
    );

    DeepValues {
        trace_at_z1: trace_state1,
        trace_at_z2: trace_state2,
    }
}

pub fn compose_constraint_poly(
    composition_poly: &mut Vec<u128>,
    constraint_poly: ConstraintPoly,
    z: u128,
    cc: &CompositionCoefficients,
) -> u128 {
    let mut constraint_poly = constraint_poly.into_vec();

    // evaluate the polynomial at point z
    let value_at_z = polynom::eval(&constraint_poly, z);

    // compute C(x) = (P(x) - P(z)) / (x - z)
    constraint_poly[0] = field::sub(constraint_poly[0], value_at_z);
    polynom::syn_div_in_place(&mut constraint_poly, z);

    // add C(x) * cc into the result
    utils::mul_acc(composition_poly, &constraint_poly, cc.constraints);

    value_at_z
}

// HELPER FUNCTIONS
// ================================================================================================

/// Computes (P(x) - v) * coeff and saves the result into the accumulator
fn acc_poly(accumulator: &mut Vec<u128>, poly: &Vec<u128>, value: u128, coeff: u128) {
    utils::mul_acc(accumulator, poly, coeff);
    let adjusted_tz = field::mul(value, coeff);
    accumulator[0] = field::sub(accumulator[0], adjusted_tz);
}

fn get_incremental_trace_degree(composition_degree: usize, trace_length: usize) -> usize {
    composition_degree - (trace_length - 2)
}
