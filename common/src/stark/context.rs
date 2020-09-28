use super::ProofOptions;
use math::field;
use std::cmp;

// TYPES AND INTERFACES
// ================================================================================================

#[derive(Clone)]
pub struct ProofContext {
    options: ProofOptions,
    trace_width: usize,
    trace_length: usize,
    ce_blowup_factor: usize,
    max_constraint_degree: usize,
    generators: Generators,
}

#[derive(Clone)]
pub struct Generators {
    pub trace_domain: u128,
    pub ce_domain: u128,
    pub lde_domain: u128,
}

// PROOF CONTEXT
// ================================================================================================

impl ProofContext {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------
    pub const MAX_FRI_REMAINDER_LENGTH: usize = 256;
    pub const FRI_FOLDING_FACTOR: usize = 4;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    pub fn new(
        trace_width: usize,
        trace_length: usize,
        max_constraint_degree: usize,
        options: ProofOptions,
    ) -> Self {
        // trace domain generator
        let g_trace = field::get_root_of_unity(trace_length);

        // constraint evaluation domain generator
        let ce_domain_size = compute_ce_domain_size(trace_length, max_constraint_degree);
        let g_ce = field::get_root_of_unity(ce_domain_size);

        // low-degree extension domain generator
        let ce_blowup_factor = cmp::max(max_constraint_degree, 2).next_power_of_two();
        let lde_domain_size = compute_lde_domain_size(trace_length, options.blowup_factor());
        let g_lde = field::get_root_of_unity(lde_domain_size);

        ProofContext {
            options,
            trace_width,
            trace_length,
            ce_blowup_factor,
            max_constraint_degree,
            generators: Generators {
                trace_domain: g_trace,
                ce_domain: g_ce,
                lde_domain: g_lde,
            },
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

    pub fn max_constraint_degree(&self) -> usize {
        self.max_constraint_degree
    }

    // OTHER PROPERTIES
    // --------------------------------------------------------------------------------------------

    pub fn num_fri_layers(&self) -> usize {
        let mut result = 0;
        let mut domain_size = self.lde_domain_size();

        while domain_size > Self::MAX_FRI_REMAINDER_LENGTH {
            domain_size /= Self::FRI_FOLDING_FACTOR;
            result += 1;
        }

        result
    }

    pub fn options(&self) -> &ProofOptions {
        &self.options
    }

    pub fn generators(&self) -> &Generators {
        &self.generators
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
