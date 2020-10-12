use super::{
    types::{CompositionPoly, ConstraintPoly, LdeDomain, PolyTable},
    utils,
};
use common::stark::{CompositionCoefficients, DeepValues};
use math::{
    fft,
    field::{f128::FieldElement, StarkField},
    polynom,
};

// PROCEDURES
// ================================================================================================

/// Combines all trace polynomials into a single polynomial and saves the result into
/// the composition polynomial. The combination is done as follows:
/// 1. First, state of trace registers at deep points z and z * g are computed;
/// 2. Then, polynomials T1_i(x) = (T_i(x) - T_i(z)) / (x - z) and
/// T2_i(x) = (T_i(x) - T_i(z * g)) / (x - z * g) are computed for all i and combined
/// together into a single polynomial using a pseudo-random linear combination;
/// 3. Then the degree of the polynomial is adjusted to match the composition degree.
pub fn compose_trace_polys(
    composition_poly: &mut CompositionPoly,
    trace_polys: PolyTable,
    z: FieldElement,
    cc: &CompositionCoefficients,
) -> DeepValues {
    // compute a second out-of-domain point which corresponds to the next
    // computation state in relation to point z
    let trace_length = trace_polys.poly_size();
    let g = FieldElement::get_root_of_unity(trace_length.trailing_zeros() as usize);
    let next_z = z * g;

    // compute state of registers at deep points z and z * g
    let trace_state1 = trace_polys.evaluate_at(z);
    let trace_state2 = trace_polys.evaluate_at(next_z);

    // combine trace polynomials into 2 composition polynomials T1(x) and T2(x)
    let polys = trace_polys.into_vec();
    let mut t1_composition = vec![FieldElement::ZERO; trace_length];
    let mut t2_composition = vec![FieldElement::ZERO; trace_length];
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

    // divide the two composition polynomials by (x - z) and (x - z * g) respectively,
    // and add the resulting polynomials together; the output of this step is a single
    // trace polynomial T(x) and deg(T(x)) = trace_length - 2
    polynom::syn_div_in_place(&mut t1_composition, z);
    polynom::syn_div_in_place(&mut t2_composition, next_z);
    utils::add_in_place(&mut t1_composition, &t2_composition);
    let trace_poly = t1_composition;
    debug_assert_eq!(trace_length - 2, polynom::degree_of(&trace_poly));

    // we need to make sure that the degree of trace polynomial T(x) matches the degree
    // of composition polynomial; to do this, we compute a linear combination of T(x)
    // with itself multiplied by x^p, where p is the incremental degree needed to match
    // the composition degree.
    let incremental_degree = composition_poly.degree() - (trace_length - 2);

    // The next few lines are an optimized way of computing:
    // C(x) = T(x) * k_1 + T(x) * x^incremental_degree * k_2
    // where k_1 and k_2 are pseudo-random coefficients.

    // this is equivalent to T(x) * k_1
    let composition_poly = composition_poly.coefficients_mut();
    utils::mul_acc(
        &mut composition_poly[..trace_length],
        &trace_poly,
        cc.t1_degree,
    );
    // this is equivalent to T(x) * x^incremental_degree * k_2
    utils::mul_acc(
        &mut composition_poly[incremental_degree..(incremental_degree + trace_length)],
        &trace_poly,
        cc.t2_degree,
    );

    // trace states at OOD points z and z * g are returned to be included in the proof
    DeepValues {
        trace_at_z1: trace_state1,
        trace_at_z2: trace_state2,
    }
}

/// Divides out OOD point z from the constraint polynomial and saves the
/// result into the composition polynomial.
pub fn compose_constraint_poly(
    composition_poly: &mut CompositionPoly,
    constraint_poly: ConstraintPoly,
    z: FieldElement,
    cc: &CompositionCoefficients,
) {
    // evaluate the polynomial at point z
    let value_at_z = constraint_poly.evaluate_at(z);

    // compute C(x) = (P(x) - P(z)) / (x - z)
    let mut constraint_poly = constraint_poly.into_vec();
    constraint_poly[0] = constraint_poly[0] - value_at_z;
    polynom::syn_div_in_place(&mut constraint_poly, z);

    // add C(x) * K into the result
    let composition_poly = composition_poly.coefficients_mut();
    utils::mul_acc(
        &mut composition_poly[..constraint_poly.len()],
        &constraint_poly,
        cc.constraints,
    );
}

/// Evaluates DEEP composition polynomial over LDE domain.
pub fn evaluate_composition_poly(
    poly: CompositionPoly,
    lde_domain: &LdeDomain,
) -> Vec<FieldElement> {
    let mut evaluations = poly.into_vec();
    fft::evaluate_poly(&mut evaluations, lde_domain.twiddles(), true);
    evaluations
}

// HELPER FUNCTIONS
// ================================================================================================

/// Computes (P(x) - value) * k and saves the result into the accumulator
fn acc_poly(
    accumulator: &mut Vec<FieldElement>,
    poly: &[FieldElement],
    value: FieldElement,
    k: FieldElement,
) {
    utils::mul_acc(accumulator, poly, k);
    let adjusted_tz = value * k;
    accumulator[0] = accumulator[0] - adjusted_tz;
}
