use super::{utils, ConstraintPoly};
use common::{errors::ProverError, ComputationContext, ConstraintDivisor};
use math::{
    fft,
    field::{BaseElement, FieldElement},
    polynom,
};

#[cfg(feature = "concurrent")]
use rayon::prelude::*;

// CONSTRAINT EVALUATION TABLE
// ================================================================================================
pub struct ConstraintEvaluationTable<E: FieldElement + From<BaseElement>> {
    evaluations: Vec<Vec<E>>,
    divisors: Vec<ConstraintDivisor>,
    composition_degree: usize,
    domain_offset: BaseElement,
}

impl<E: FieldElement + From<BaseElement>> ConstraintEvaluationTable<E> {
    pub fn new(
        evaluations: Vec<Vec<E>>,
        divisors: Vec<ConstraintDivisor>,
        context: &ComputationContext,
    ) -> Self {
        // TODO: verify lengths
        ConstraintEvaluationTable {
            evaluations,
            divisors,
            domain_offset: context.domain_offset(),
            composition_degree: context.composition_degree(),
        }
    }

    pub fn domain_size(&self) -> usize {
        self.evaluations[0].len()
    }

    #[allow(dead_code)]
    pub fn divisors(&self) -> &[ConstraintDivisor] {
        &self.divisors
    }

    // CONSTRAINT COMPOSITION
    // --------------------------------------------------------------------------------------------
    /// Interpolates all constraint evaluations into polynomials, divides them by their respective
    /// divisors, and combines the results into a single polynomial
    pub fn into_poly(self) -> Result<ConstraintPoly<E>, ProverError> {
        let constraint_poly_degree = self.composition_degree;
        let domain_offset = self.domain_offset;

        // allocate memory for the combined polynomial
        let mut combined_poly = vec![E::ZERO; self.domain_size()];

        // build twiddles for interpolation; these can be used to interpolate all polynomials
        let inv_twiddles = fft::get_inv_twiddles::<BaseElement>(self.domain_size());

        #[cfg(feature = "concurrent")]
        {
            let divisors = self.divisors;
            let polys = self
                .evaluations
                .into_par_iter()
                .zip(divisors.par_iter())
                .map(|(column, divisor)| {
                    apply_divisor(column, divisor, &inv_twiddles, domain_offset)
                })
                .collect::<Vec<_>>();

            for poly in polys.into_iter() {
                #[cfg(debug_assertions)]
                validate_degree(&poly, constraint_poly_degree)?;
                utils::add_in_place(&mut combined_poly, &poly);
            }
        }

        // iterate over all columns of the constraint evaluation table
        #[cfg(not(feature = "concurrent"))]
        for (column, divisor) in self.evaluations.into_iter().zip(self.divisors.iter()) {
            let poly = apply_divisor(column, divisor, &inv_twiddles, domain_offset);
            #[cfg(debug_assertions)]
            validate_degree(&poly, constraint_poly_degree)?;
            utils::add_in_place(&mut combined_poly, &poly);
        }

        Ok(ConstraintPoly::new(combined_poly, constraint_poly_degree))
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn apply_divisor<E>(
    mut column: Vec<E>,
    divisor: &ConstraintDivisor,
    inv_twiddles: &[BaseElement],
    domain_offset: BaseElement,
) -> Vec<E>
where
    E: FieldElement + From<BaseElement>,
{
    let numerator = divisor.numerator();
    assert!(
        numerator.len() == 1,
        "complex divisors are not yet supported"
    );
    assert!(
        divisor.exclude().len() <= 1,
        "multiple exclusion points are not yet supported"
    );

    // convert the polynomial into coefficient form by interpolating the evaluations
    // over the evaluation domain
    fft::interpolate_poly_with_offset(&mut column, &inv_twiddles, domain_offset);
    let mut poly = column;

    // divide the polynomial by its divisor
    let numerator = numerator[0];
    let degree = numerator.0;

    if divisor.exclude().is_empty() {
        // the form of the divisor is just (x^degree - a)
        let a = E::from(numerator.1);
        polynom::syn_div_in_place(&mut poly, degree, a);
    } else {
        // the form of divisor is (x^degree - 1) / (x - exception)
        let exception = E::from(divisor.exclude()[0]);
        polynom::syn_div_in_place_with_exception(&mut poly, degree, exception);
    }

    poly
}

/// makes sure that the post-division degree of the polynomial matches the expected degree
#[cfg(debug_assertions)]
fn validate_degree<E: FieldElement>(
    poly: &[E],
    composition_degree: usize,
) -> Result<(), ProverError> {
    if composition_degree != polynom::degree_of(&poly) {
        return Err(ProverError::MismatchedConstraintPolynomialDegree(
            composition_degree,
            polynom::degree_of(&poly),
        ));
    }
    Ok(())
}
