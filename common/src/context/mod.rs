use crate::ProofOptions;
use math::field::{BaseElement, FieldElement, StarkField};

// TYPES AND INTERFACES
// ================================================================================================

#[derive(Clone)]
pub struct ComputationContext {
    options: ProofOptions,
    trace_width: usize,
    trace_length: usize,
    ce_blowup_factor: usize,
    field_extension: FieldExtension,
    generators: Generators,
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum FieldExtension {
    None = 1,
    Quadratic = 2,
}

#[derive(Clone)]
pub struct Generators {
    pub trace_domain: BaseElement,
    pub ce_domain: BaseElement,
    pub lde_domain: BaseElement,
}

// COMPUTATION CONTEXT
// ================================================================================================

impl ComputationContext {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------
    pub const MIN_TRACE_LENGTH: usize = 8;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    pub fn new(
        trace_width: usize,
        trace_length: usize,
        ce_blowup_factor: usize,
        field_extension: FieldExtension,
        options: ProofOptions,
    ) -> Self {
        assert!(
            trace_width > 0,
            "trace_width must be greater than 0; was {}",
            trace_width
        );
        assert!(
            trace_length >= Self::MIN_TRACE_LENGTH,
            "trace_length must beat least {}; was {}",
            Self::MIN_TRACE_LENGTH,
            trace_length
        );
        assert!(
            trace_length.is_power_of_two(),
            "trace_length must be a power of 2; was {}",
            trace_length
        );
        assert!(
            ce_blowup_factor > 1,
            "ce_blowup_factor must be greater than 1; was {}",
            ce_blowup_factor
        );
        assert!(
            ce_blowup_factor.is_power_of_two(),
            "ce_blowup_factor must be a power of 2; was {}",
            ce_blowup_factor
        );

        let generators = build_generators(trace_length, ce_blowup_factor, options.blowup_factor());

        ComputationContext {
            options,
            trace_width,
            trace_length,
            ce_blowup_factor,
            field_extension,
            generators,
        }
    }

    // TRACE INFO
    // --------------------------------------------------------------------------------------------

    pub fn trace_width(&self) -> usize {
        self.trace_width
    }

    pub fn trace_length(&self) -> usize {
        self.trace_length
    }

    pub fn trace_poly_degree(&self) -> usize {
        self.trace_length - 1
    }

    // CONSTRAINT INFO
    // --------------------------------------------------------------------------------------------

    pub fn lde_domain_size(&self) -> usize {
        compute_lde_domain_size(self.trace_length, self.options.blowup_factor())
    }

    pub fn ce_domain_size(&self) -> usize {
        compute_ce_domain_size(self.trace_length, self.ce_blowup_factor)
    }

    pub fn ce_blowup_factor(&self) -> usize {
        self.ce_blowup_factor
    }

    pub fn composition_degree(&self) -> usize {
        self.ce_domain_size() - self.trace_length
    }

    pub fn deep_composition_degree(&self) -> usize {
        self.composition_degree() - 1
    }

    // OTHER PROPERTIES
    // --------------------------------------------------------------------------------------------

    pub fn field_extension(&self) -> FieldExtension {
        self.field_extension
    }

    pub fn options(&self) -> &ProofOptions {
        &self.options
    }

    pub fn generators(&self) -> &Generators {
        &self.generators
    }

    // UTILITY FUNCTIONS
    // --------------------------------------------------------------------------------------------

    /// Returns g^step, where g is the generator of trace domain.
    pub fn get_trace_x_at(&self, step: usize) -> BaseElement {
        debug_assert!(
            step < self.trace_length,
            "step must be in the trace domain [0, {})",
            self.trace_length
        );
        self.generators.trace_domain.exp((step as u64).into())
    }

    /// Returns a sequence: g^first_step, g^(first_step + stride), g^(first_step + 2 * stride)...
    /// where g is the generator of trace domain. The number of elements in the sequence is
    /// defined as trace_length / stride.
    pub fn get_trace_x_at_steps(&self, first_step: usize, stride: usize) -> Vec<BaseElement> {
        debug_assert!(
            stride.is_power_of_two(),
            "stride must be a power of two but was {}",
            stride
        );
        debug_assert!(
            first_step < stride,
            "first step ({}) cannot be greater than stride ({})",
            first_step,
            stride
        );
        debug_assert!(
            stride < self.trace_length,
            "stride ({}) must be smaller than trace length ({})",
            stride,
            self.trace_length
        );

        // compute g^first_step and g^stride
        let start = self.generators.trace_domain.exp((first_step as u64).into());
        let step = self.generators.trace_domain.exp((stride as u64).into());

        let mut result = vec![BaseElement::ZERO; self.trace_length / stride];
        result[0] = start;
        for i in 1..result.len() {
            result[i] = result[i - 1] * step;
        }

        result
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn compute_lde_domain_size(trace_length: usize, lde_blowup_factor: usize) -> usize {
    trace_length * lde_blowup_factor
}

fn compute_ce_domain_size(trace_length: usize, ce_blowup_factor: usize) -> usize {
    trace_length * ce_blowup_factor
}

fn build_generators(
    trace_length: usize,
    ce_blowup_factor: usize,
    lde_blowup_factor: usize,
) -> Generators {
    let ce_domain_size = compute_ce_domain_size(trace_length, ce_blowup_factor);
    let lde_domain_size = compute_lde_domain_size(trace_length, lde_blowup_factor);

    Generators {
        trace_domain: BaseElement::get_root_of_unity(trace_length.trailing_zeros()),
        ce_domain: BaseElement::get_root_of_unity(ce_domain_size.trailing_zeros()),
        lde_domain: BaseElement::get_root_of_unity(lde_domain_size.trailing_zeros()),
    }
}
