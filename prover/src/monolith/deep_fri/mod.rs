use super::{constraints::ConstraintPoly, types::PolyTable, StarkDomain};
use common::{CompositionCoefficients, ComputationContext, EvaluationFrame};
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
    polynom, utils,
};

// COMPOSITION POLYNOMIAL
// ================================================================================================
pub struct CompositionPoly<E: FieldElement + From<BaseElement>> {
    coefficients: Vec<E>,
    degree: usize,
    cc: CompositionCoefficients<E>,
    z: E,
    next_z: E,
}

impl<E: FieldElement + From<BaseElement>> CompositionPoly<E> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    pub fn new(context: &ComputationContext, z: E, cc: CompositionCoefficients<E>) -> Self {
        let g = E::from(BaseElement::get_root_of_unity(
            context.trace_length().trailing_zeros(),
        ));
        let next_z = z * g;

        CompositionPoly {
            coefficients: E::zeroed_vector(context.ce_domain_size()),
            degree: context.deep_composition_degree(),
            cc,
            z,
            next_z,
        }
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    pub fn degree(&self) -> usize {
        self.degree
    }

    #[allow(dead_code)] // TODO: remove
    pub fn len(&self) -> usize {
        self.coefficients.len()
    }

    // TRACE POLYNOMIAL COMPOSITION
    // --------------------------------------------------------------------------------------------
    /// Combines all trace polynomials into a single polynomial and saves the result into
    /// the composition polynomial. The combination is done as follows:
    /// 1. First, state of trace registers at deep points z and z * g are computed;
    /// 2. Then, polynomials T1_i(x) = (T_i(x) - T_i(z)) / (x - z) and
    /// T2_i(x) = (T_i(x) - T_i(z * g)) / (x - z * g) are computed for all i and combined
    /// together into a single polynomial using a pseudo-random linear combination;
    /// 3. Then the degree of the polynomial is adjusted to match the composition degree.
    pub fn add_trace_polys(&mut self, trace_polys: PolyTable) -> EvaluationFrame<E> {
        // compute a second out-of-domain point which corresponds to the next
        // computation state in relation to point z
        let trace_length = trace_polys.poly_size();

        // compute state of registers at deep points z and z * g
        let trace_state1 = trace_polys.evaluate_at(self.z);
        let trace_state2 = trace_polys.evaluate_at(self.next_z);

        // combine trace polynomials into 2 composition polynomials T1(x) and T2(x)
        let polys = trace_polys.into_vec();
        let mut t1_composition = E::zeroed_vector(trace_length);
        let mut t2_composition = E::zeroed_vector(trace_length);
        for (i, poly) in polys.into_iter().enumerate() {
            // Convert polys[i] from type BaseElement into type E
            // TODO: find a better ay to do this (ideally, with zero-copy)
            let e_poly = poly.into_iter().map(E::from).collect::<Vec<_>>();

            // compute T1(x) = (T(x) - T(z)), multiply it by a pseudo-random
            // coefficient, and add the result into composition polynomial
            acc_poly(
                &mut t1_composition,
                &e_poly,
                trace_state1[i],
                self.cc.trace[i].0,
            );

            // compute T2(x) = (T(x) - T(z * g)), multiply it by a pseudo-random
            // coefficient, and add the result into composition polynomial
            acc_poly(
                &mut t2_composition,
                &e_poly,
                trace_state2[i],
                self.cc.trace[i].1,
            );
        }

        // divide the two composition polynomials by (x - z) and (x - z * g) respectively,
        // and add the resulting polynomials together; the output of this step is a single
        // trace polynomial T(x) and deg(T(x)) = trace_length - 2
        polynom::syn_div_in_place(&mut t1_composition, 1, self.z);
        polynom::syn_div_in_place(&mut t2_composition, 1, self.next_z);
        utils::add_in_place(&mut t1_composition, &t2_composition);
        let trace_poly = t1_composition;
        debug_assert_eq!(trace_length - 2, polynom::degree_of(&trace_poly));

        // we need to make sure that the degree of trace polynomial T(x) matches the degree
        // of composition polynomial; to do this, we compute a linear combination of T(x)
        // with itself multiplied by x^p, where p is the incremental degree needed to match
        // the composition degree.
        let incremental_degree = self.degree() - (trace_length - 2);

        // The next few lines are an optimized way of computing:
        // C(x) = T(x) * k_1 + T(x) * x^incremental_degree * k_2
        // where k_1 and k_2 are pseudo-random coefficients.

        // this is equivalent to T(x) * k_1
        utils::mul_acc(
            &mut self.coefficients[..trace_length],
            &trace_poly,
            self.cc.trace_degree.0,
        );
        // this is equivalent to T(x) * x^incremental_degree * k_2
        utils::mul_acc(
            &mut self.coefficients[incremental_degree..(incremental_degree + trace_length)],
            &trace_poly,
            self.cc.trace_degree.1,
        );

        // trace states at OOD points z and z * g are returned to be included in the proof
        EvaluationFrame {
            current: trace_state1,
            next: trace_state2,
        }
    }

    // CONSTRAINT POLYNOMIAL COMPOSITION
    // --------------------------------------------------------------------------------------------
    /// Divides out OOD point z from the constraint polynomial and saves the
    /// result into the composition polynomial.
    pub fn add_constraint_poly(&mut self, constraint_poly: ConstraintPoly<BaseElement>) {
        // TODO: find a better ay to do this (ideally, with zero-copy)
        let mut constraint_poly = constraint_poly
            .into_vec()
            .into_iter()
            .map(E::from)
            .collect::<Vec<_>>();

        // evaluate the polynomial at point z
        let value_at_z = polynom::eval(&constraint_poly, self.z);

        // compute C(x) = (P(x) - P(z)) / (x - z)
        constraint_poly[0] = constraint_poly[0] - value_at_z;
        polynom::syn_div_in_place(&mut constraint_poly, 1, self.z);

        // add C(x) * K into the result
        utils::mul_acc(
            &mut self.coefficients[..constraint_poly.len()],
            &constraint_poly,
            self.cc.constraints,
        );
    }

    // LOW-DEGREE EXTENSION
    // --------------------------------------------------------------------------------------------
    /// Evaluates DEEP composition polynomial over the specified LDE domain and returns the result.
    pub fn evaluate(self, domain: &StarkDomain) -> Vec<E> {
        fft::evaluate_poly_with_offset(
            &self.coefficients,
            domain.ce_twiddles(),
            domain.offset(),
            domain.ce_to_lde_blowup(),
        )
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Computes (P(x) - value) * k and saves the result into the accumulator
fn acc_poly<E: FieldElement>(accumulator: &mut Vec<E>, poly: &[E], value: E, k: E) {
    utils::mul_acc(accumulator, poly, k);
    let adjusted_tz = value * k;
    accumulator[0] = accumulator[0] - adjusted_tz;
}
