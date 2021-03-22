use super::super::ComputationDomain;
use common::ComputationContext;
use math::{
    fft,
    field::{BaseElement, FieldElement},
};

// COMPOSITION POLYNOMIAL
// ================================================================================================
pub struct CompositionPoly<E: FieldElement + From<BaseElement>>(Vec<E>, usize);

impl<E: FieldElement + From<BaseElement>> CompositionPoly<E> {
    pub fn new(context: &ComputationContext) -> Self {
        CompositionPoly(
            vec![E::ZERO; context.ce_domain_size()],
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

    pub fn coefficients_mut(&mut self) -> &mut [E] {
        &mut self.0
    }

    /// Evaluates DEEP composition polynomial over LDE domain.
    pub fn evaluate(self, domain: &ComputationDomain) -> Vec<E> {
        fft::evaluate_poly_with_offset(
            &self.0,
            domain.ce_twiddles(),
            domain.offset(),
            domain.ce_to_lde_blowup(),
        )
    }
}
