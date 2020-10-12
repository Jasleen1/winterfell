use common::stark::{ConstraintDivisor, ProofContext};
use math::{polynom, field::{StarkField, f128::FieldElement}};
use std::{iter, vec};

// TRACE TABLE
// ================================================================================================
pub struct TraceTable(Vec<Vec<FieldElement>>);

impl TraceTable {
    pub fn new(registers: Vec<Vec<FieldElement>>) -> Self {
        assert!(
            !registers.is_empty(),
            "execution trace must consist of at least one register"
        );
        let trace_length = registers[0].len();
        assert!(
            trace_length.is_power_of_two(),
            "execution trace length must be a power of 2"
        );
        for register in registers.iter() {
            assert!(
                register.len() == trace_length,
                "all register traces must have the same length"
            );
        }

        TraceTable(registers)
    }

    pub fn num_states(&self) -> usize {
        self.0[0].len()
    }

    pub fn num_registers(&self) -> usize {
        self.0.len()
    }

    pub fn copy_row(&self, idx: usize, destination: &mut [FieldElement]) {
        for (i, register) in self.0.iter().enumerate() {
            destination[i] = register[idx];
        }
    }

    pub fn get(&self, register: usize, step: usize) -> FieldElement {
        self.0[register][step]
    }

    #[cfg(test)]
    pub fn get_register(&self, idx: usize) -> &[FieldElement] {
        &self.0[idx]
    }

    pub fn into_vec(self) -> Vec<Vec<FieldElement>> {
        self.0
    }
}

// LOW DEGREE EXTENSION DOMAIN
// ================================================================================================
pub struct LdeDomain(Vec<FieldElement>, Vec<FieldElement>);

impl LdeDomain {
    pub fn new(values: Vec<FieldElement>, twiddles: Vec<FieldElement>) -> Self {
        assert!(
            values.len().is_power_of_two(),
            "Size of LDE domain must be a power of 2"
        );
        assert!(
            twiddles.len() * 2 == values.len(),
            "Twiddles must be half the size of the domain"
        );
        LdeDomain(values, twiddles)
    }

    pub fn size(&self) -> usize {
        self.0.len()
    }

    pub fn twiddles(&self) -> &[FieldElement] {
        &self.1
    }

    pub fn values(&self) -> &[FieldElement] {
        &self.0
    }
}

// POLYNOMIAL TABLE
// ================================================================================================
pub struct PolyTable(Vec<Vec<FieldElement>>);

impl PolyTable {
    pub fn new(polys: Vec<Vec<FieldElement>>) -> Self {
        assert!(
            !polys.is_empty(),
            "polynomial table must contain at least one polynomial"
        );
        let poly_size = polys[0].len();
        assert!(
            poly_size.is_power_of_two(),
            "polynomial size must be a power of 2"
        );
        for poly in polys.iter() {
            assert!(
                poly.len() == poly_size,
                "all polynomials must have the same size"
            );
        }

        PolyTable(polys)
    }

    pub fn poly_size(&self) -> usize {
        self.0[0].len()
    }

    /// Evaluates all polynomials the the specified point `x`.
    pub fn evaluate_at(&self, x: FieldElement) -> Vec<FieldElement> {
        let mut result = Vec::with_capacity(self.num_polys());
        for poly in self.0.iter() {
            result.push(polynom::eval(&poly, x));
        }
        result
    }

    pub fn num_polys(&self) -> usize {
        self.0.len()
    }

    #[cfg(test)]
    pub fn get_poly(&self, idx: usize) -> &[FieldElement] {
        &self.0[idx]
    }

    pub fn into_vec(self) -> Vec<Vec<FieldElement>> {
        self.0
    }
}

// CONSTRAINT EVALUATION TABLE
// ================================================================================================
#[allow(dead_code)]
pub struct ConstraintEvaluationTable {
    evaluations: Vec<Vec<FieldElement>>,
    divisors: Vec<ConstraintDivisor>,
}

impl ConstraintEvaluationTable {
    pub fn new(evaluations: Vec<Vec<FieldElement>>, divisors: Vec<ConstraintDivisor>) -> Self {
        // TODO: verify lengths
        ConstraintEvaluationTable {
            evaluations,
            divisors,
        }
    }

    pub fn domain_size(&self) -> usize {
        self.evaluations[0].len()
    }

    pub fn divisors(&self) -> &[ConstraintDivisor] {
        &self.divisors
    }

    pub fn into_vec(self) -> Vec<Vec<FieldElement>> {
        self.evaluations
    }
}

impl IntoIterator for ConstraintEvaluationTable {
    type Item = (Vec<FieldElement>, ConstraintDivisor);
    type IntoIter = iter::Zip<vec::IntoIter<Vec<FieldElement>>, vec::IntoIter<ConstraintDivisor>>;

    fn into_iter(self) -> Self::IntoIter {
        self.evaluations.into_iter().zip(self.divisors.into_iter())
    }
}

// CONSTRAINT POLYNOMIAL
// ================================================================================================
pub struct ConstraintPoly(Vec<FieldElement>, usize);

impl ConstraintPoly {
    pub fn new(coefficients: Vec<FieldElement>, degree: usize) -> Self {
        ConstraintPoly(coefficients, degree)
    }

    pub fn degree(&self) -> usize {
        self.1
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn coefficients(&self) -> &[FieldElement] {
        &self.0
    }

    /// Evaluates the polynomial the the specified point `x`.
    pub fn evaluate_at(&self, x: FieldElement) -> FieldElement {
        polynom::eval(&self.0, x)
    }

    pub fn into_vec(self) -> Vec<FieldElement> {
        self.0
    }
}

// COMPOSITION POLYNOMIAL
// ================================================================================================
pub struct CompositionPoly(Vec<FieldElement>, usize);

impl CompositionPoly {
    pub fn new(context: &ProofContext) -> Self {
        CompositionPoly(
            vec![FieldElement::ZERO; context.lde_domain_size()],
            context.deep_composition_degree(),
        )
    }

    pub fn degree(&self) -> usize {
        self.1
    }

    #[allow(dead_code)] // TODO: remove
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn coefficients_mut(&mut self) -> &mut [FieldElement] {
        &mut self.0
    }

    pub fn into_vec(self) -> Vec<FieldElement> {
        self.0
    }
}
