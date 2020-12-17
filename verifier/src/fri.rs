use super::channel::VerifierChannel;
use common::{fri_utils::get_augmented_positions, ComputationContext, PublicCoin};

use math::{
    field::{BaseElement, FieldElement},
    polynom, quartic,
};
use std::mem;

// VERIFIER
// ================================================================================================

/// Returns OK(true) if values in the `evaluations` slice represent evaluations of a polynomial
/// with degree <= context.deep_composition_degree() against x coordinates specified by
/// `positions` slice
pub fn verify<E: FieldElement + From<BaseElement>>(
    context: &ComputationContext,
    channel: &VerifierChannel,
    evaluations: &[E],
    positions: &[usize],
) -> Result<bool, String> {
    let max_degree = context.deep_composition_degree();
    let domain_size = context.lde_domain_size();
    let domain_root = context.generators().lde_domain;

    // powers of the given root of unity 1, p, p^2, p^3 such that p^4 = 1
    let root_1_modp4: u32 = (domain_size / 4) as u32;
    let root_2_modp4: u32 = (domain_size / 2) as u32;
    let root_3_modp4: u32 = (domain_size * 3 / 4) as u32;

    let quartic_roots = [
        BaseElement::ONE,
        BaseElement::exp(domain_root, root_1_modp4.into()),
        BaseElement::exp(domain_root, root_2_modp4.into()),
        BaseElement::exp(domain_root, root_3_modp4.into()),
    ];

    // 1 ----- verify the recursive components of the FRI proof -----------------------------------
    let mut domain_root = domain_root;
    let mut domain_size = domain_size;
    let mut max_degree_plus_1 = max_degree + 1;
    let mut positions = positions.to_vec();
    let mut evaluations = evaluations.to_vec();

    for depth in 0..context.num_fri_layers() {
        let mut augmented_positions = get_augmented_positions(&positions, domain_size);
        let layer_values = channel.read_fri_queries(depth, &augmented_positions)?;
        let column_values =
            get_column_values(&layer_values, &positions, &augmented_positions, domain_size);
        if evaluations != column_values {
            return Err(format!(
                "evaluations did not match column value at depth {}",
                depth
            ));
        }

        // build a set of x for each row polynomial
        let mut xs = Vec::with_capacity(augmented_positions.len());
        for &i in augmented_positions.iter() {
            let i_as_32 = i as u32;
            let xe = BaseElement::exp(domain_root, i_as_32.into());
            xs.push([
                quartic_roots[0] * xe,
                quartic_roots[1] * xe,
                quartic_roots[2] * xe,
                quartic_roots[3] * xe,
            ]);
        }

        // interpolate x and y values into row polynomials
        let row_polys = quartic::interpolate_batch(&xs, &layer_values);

        // calculate the pseudo-random x coordinate
        let special_x = channel.draw_fri_point(depth);

        // check that when the polynomials are evaluated at x, the result is equal to
        // the corresponding column value
        evaluations = quartic::evaluate_batch(&row_polys, special_x);

        // update variables for the next iteration of the loop
        domain_root = BaseElement::exp(domain_root, 4);
        max_degree_plus_1 /= 4;
        domain_size /= 4;
        mem::swap(&mut positions, &mut augmented_positions);
    }

    // 2 ----- verify the remainder of the FRI proof ----------------------------------------------

    // read the remainder from the channel and make sure it matches with the columns
    // of the previous layer
    let remainder = channel.read_fri_remainder::<E>()?;
    for (&position, evaluation) in positions.iter().zip(evaluations) {
        if remainder[position] != evaluation {
            return Err(String::from(
                "remainder values are inconsistent with values of the last column",
            ));
        }
    }

    // make sure the remainder values satisfy the degree
    verify_remainder(
        remainder,
        max_degree_plus_1,
        domain_root,
        context.options().blowup_factor(),
    )
}

/// Returns Ok(true) if values in the `remainder` slice represent evaluations of a polynomial
/// with degree < max_degree_plus_1 against a domain specified by the `domain_root`
fn verify_remainder<E: FieldElement + From<BaseElement>>(
    remainder: Vec<E>,
    max_degree_plus_1: usize,
    domain_root: BaseElement,
    blowup_factor: usize,
) -> Result<bool, String> {
    if max_degree_plus_1 > remainder.len() {
        return Err(String::from(
            "remainder degree is greater than number of remainder values",
        ));
    }

    // exclude points which should be skipped during evaluation
    let mut positions = Vec::new();
    for i in 0..remainder.len() {
        if i % blowup_factor != 0 {
            positions.push(i);
        }
    }

    // pick a subset of points from the remainder and interpolate them into a polynomial
    let domain = BaseElement::get_power_series(domain_root, remainder.len());
    let mut xs = Vec::with_capacity(max_degree_plus_1);
    let mut ys = Vec::with_capacity(max_degree_plus_1);
    for &p in positions.iter().take(max_degree_plus_1) {
        xs.push(E::from(domain[p]));
        ys.push(remainder[p]);
    }
    let poly = polynom::interpolate(&xs, &ys, false);

    // check that polynomial evaluates correctly for all other points in the remainder
    for &p in positions.iter().skip(max_degree_plus_1) {
        if polynom::eval(&poly, E::from(domain[p])) != remainder[p] {
            return Err(format!(
                "remainder is not a valid degree {} polynomial",
                max_degree_plus_1 - 1
            ));
        }
    }

    Ok(true)
}

// HELPER FUNCTIONS
// ================================================================================================
fn get_column_values<E: FieldElement>(
    values: &[[E; 4]],
    positions: &[usize],
    augmented_positions: &[usize],
    column_length: usize,
) -> Vec<E> {
    let row_length = column_length / 4;

    let mut result = Vec::new();
    for position in positions {
        let idx = augmented_positions
            .iter()
            .position(|&v| v == position % row_length)
            .unwrap();
        let value = values[idx][position / row_length];
        result.push(value);
    }

    result
}

// TESTS
// ================================================================================================
#[cfg(test)]
mod tests {

    /*
    // TODO
    #[test]
    fn verify_remainder() {
        let degree_plus_1: usize = 32;
        let root = field::get_root_of_unity(degree_plus_1 * 2);
        let extension_factor = 16;

        let mut remainder = field::rand_vector(degree_plus_1);
        remainder.resize(degree_plus_1 * 2, 0);
        polynom::eval_fft(&mut remainder, true);

        // check against exact degree
        let result = super::verify_remainder(&remainder, degree_plus_1, root, extension_factor);
        assert_eq!(Ok(true), result);

        // check against higher degree
        let result = super::verify_remainder(&remainder, degree_plus_1 + 1, root, extension_factor);
        assert_eq!(Ok(true), result);

        // check against lower degree
        let degree_plus_1 = degree_plus_1 - 1;
        let result = super::verify_remainder(&remainder, degree_plus_1, root, extension_factor);
        let err_msg = format!("remainder is not a valid degree {} polynomial", degree_plus_1 - 1);
        assert_eq!(Err(err_msg), result);
    }
    */
}
