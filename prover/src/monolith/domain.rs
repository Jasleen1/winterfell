use common::ComputationContext;
use math::{
    fft,
    field::{BaseElement, StarkField},
};

// TYPES AND INTERFACES
// ================================================================================================

pub struct ComputationDomain {
    /// Contains all values in the low-degree extension domain. Length of this vector is the same
    /// as the size of LDE domain.
    lde_domain: Vec<BaseElement>,

    /// Twiddles which can be used to evaluate polynomials in the trace domain. Length of this
    /// vector is half the length of the trace domain size.
    trace_twiddles: Vec<BaseElement>,

    /// Twiddles which can be used to evaluate polynomials in the constraint evaluation domain.
    /// Length of this vector is half the length of constraint evaluation domain size.
    ce_twiddles: Vec<BaseElement>,
}

// COMPUTATION DOMAIN IMPLEMENTATION
// ================================================================================================

impl ComputationDomain {
    /// Returns a new computation domain initialized with the provided `context`.
    pub fn new(context: &ComputationContext) -> Self {
        let lde_domain = build_lde_domain(context.lde_domain_size(), context.domain_offset());
        let trace_twiddles = fft::get_twiddles(context.trace_length());
        let ce_twiddles = fft::get_twiddles(context.ce_domain_size());
        ComputationDomain {
            lde_domain,
            trace_twiddles,
            ce_twiddles,
        }
    }

    // EXECUTION TRACE
    // --------------------------------------------------------------------------------------------

    /// Returns length of the execution trace for this computation.
    pub fn trace_length(&self) -> usize {
        &self.trace_twiddles.len() * 2
    }

    /// Returns twiddles which can be used to evaluate trace polynomials.
    pub fn trace_twiddles(&self) -> &[BaseElement] {
        &self.trace_twiddles
    }

    /// Returns blowup factor from trace to constraint evaluation domain.
    #[allow(dead_code)]
    pub fn trace_to_ce_blowup(&self) -> usize {
        self.ce_domain_size() / self.trace_length()
    }

    /// Returns blowup factor from trace to LDE domain.
    pub fn trace_to_lde_blowup(&self) -> usize {
        self.lde_domain_size() / self.trace_length()
    }

    // CONSTRAINT EVALUATION DOMAIN
    // --------------------------------------------------------------------------------------------

    /// Returns the size of the constraint evaluation domain for this computation.
    pub fn ce_domain_size(&self) -> usize {
        &self.ce_twiddles.len() * 2
    }

    /// Returns twiddles which can be used to evaluate constraint polynomials.
    pub fn ce_twiddles(&self) -> &[BaseElement] {
        &self.ce_twiddles
    }

    /// Returns blowup factor from constraint evaluation to LDE domain.
    pub fn ce_to_lde_blowup(&self) -> usize {
        self.lde_domain_size() / self.ce_domain_size()
    }

    // LOW-DEGREE EXTENSION DOMAIN
    // --------------------------------------------------------------------------------------------

    /// Returns the size of the low-degree extension domain.
    pub fn lde_domain_size(&self) -> usize {
        self.lde_domain.len()
    }

    /// Returns all values in the LDE domain.
    pub fn lde_values(&self) -> &[BaseElement] {
        &self.lde_domain
    }

    /// Returns LDE domain offset.
    pub fn offset(&self) -> BaseElement {
        self.lde_domain[0]
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_lde_domain<B: StarkField>(domain_size: usize, offset: B) -> Vec<B> {
    let g = B::get_root_of_unity(domain_size.trailing_zeros());
    B::get_power_series_with_offset(g, offset, domain_size)
}
