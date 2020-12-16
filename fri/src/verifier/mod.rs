use crate::utils::get_augmented_positions;
use math::{
    field::{BaseElement, FieldElement},
    polynom, quartic,
};
use std::mem;

mod context;
pub use context::VerifierContext;

mod channel;
pub use channel::{DefaultVerifierChannel, VerifierChannel};

// VERIFICATION PROCEDURE
// ================================================================================================

/// Returns OK(true) if values in the `evaluations` slice represent evaluations of a polynomial
/// with degree <= context.max_degree() against x coordinates specified by the `positions` slice.
pub fn verify<E, C>(
    context: &VerifierContext,
    channel: &C,
    evaluations: &[E],
    positions: &[usize],
) -> Result<bool, String>
where
    E: FieldElement + From<BaseElement>,
    C: VerifierChannel,
{
    let domain_size = context.domain_size();
    let domain_root = context.domain_root();

    // powers of the given root of unity 1, p, p^2, p^3 such that p^4 = 1
    let quartic_roots = [
        BaseElement::ONE,
        BaseElement::exp(domain_root, (domain_size as u32 / 4).into()),
        BaseElement::exp(domain_root, (domain_size as u32 / 2).into()),
        BaseElement::exp(domain_root, (domain_size as u32 * 3 / 4).into()),
    ];

    // 1 ----- verify the recursive components of the FRI proof -----------------------------------
    let mut domain_root = domain_root;
    let mut domain_size = domain_size;
    let mut max_degree_plus_1 = context.max_degree() + 1;
    let mut positions = positions.to_vec();
    let mut evaluations = evaluations.to_vec();

    for depth in 0..context.num_fri_layers() {
        let mut augmented_positions = get_augmented_positions(&positions, domain_size);
        let layer_values = channel.read_layer_queries(depth, &augmented_positions)?;
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
            let xe = BaseElement::exp(domain_root, (i as u32).into());
            xs.push([
                quartic_roots[0] * xe,
                quartic_roots[1] * xe,
                quartic_roots[2] * xe,
                quartic_roots[3] * xe,
            ]);
        }

        // interpolate x and y values into row polynomials
        let row_polys = quartic::interpolate_batch(&xs, &layer_values);

        // calculate the pseudo-random value used for linear combination in layer folding
        let alpha = channel.draw_fri_alpha(depth);

        // check that when the polynomials are evaluated at alpha, the result is equal to
        // the corresponding column value
        evaluations = quartic::evaluate_batch(&row_polys, alpha);

        // update variables for the next iteration of the loop
        domain_root = BaseElement::exp(domain_root, 4);
        max_degree_plus_1 /= 4;
        domain_size /= 4;
        mem::swap(&mut positions, &mut augmented_positions);
    }

    // 2 ----- verify the remainder of the FRI proof ----------------------------------------------

    // read the remainder from the channel and make sure it matches with the columns
    // of the previous layer
    let remainder = channel.read_remainder::<E>()?;
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
        context.blowup_factor(),
    )
}

/// Returns Ok(true) if values in the `remainder` slice represent evaluations of a polynomial
/// with degree < max_degree_plus_1 against a domain specified by the `domain_root`.
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
