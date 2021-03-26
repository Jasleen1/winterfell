use super::{trace_table::TraceTable, PolyTable, StarkDomain};
use common::Assertions;
use math::{fft, field::BaseElement};

#[cfg(feature = "concurrent")]
use rayon::prelude::*;

// CONSTANTS
// ================================================================================================

const MIN_TRACE_LENGTH: usize = 8;

// TRACE TABLE
// ================================================================================================
pub struct ExecutionTrace(Vec<Vec<BaseElement>>);

impl ExecutionTrace {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new trace table from a list of provided register traces.
    pub fn init(registers: Vec<Vec<BaseElement>>) -> Self {
        assert!(
            !registers.is_empty(),
            "execution trace must consist of at least one register"
        );
        let trace_length = registers[0].len();
        assert!(
            trace_length >= MIN_TRACE_LENGTH,
            "execution trace must be at lest {} steps long, but was {}",
            MIN_TRACE_LENGTH,
            trace_length
        );
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

        ExecutionTrace(registers)
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns number of registers in the trace table.
    pub fn width(&self) -> usize {
        self.0.len()
    }

    /// Returns the number of states in this trace table.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0[0].len()
    }

    /// Returns value in the specified `register` at the specified `step`.
    pub fn get(&self, register: usize, step: usize) -> BaseElement {
        self.0[register][step]
    }

    /// Returns the entire register trace for the register at the specified index.
    pub fn get_register(&self, idx: usize) -> &[BaseElement] {
        &self.0[idx]
    }

    // VALIDATION
    // --------------------------------------------------------------------------------------------
    pub fn validate_assertions(&self, assertions: &Assertions) {
        // TODO: eventually, this should return errors instead of panicking
        assert!(
            !assertions.is_empty(),
            "at least one assertion must be provided"
        );

        assertions.for_each(|register, step, value| {
            assert!(
                value == self.get(register, step),
                "trace does not satisfy assertion trace({}, {}) == {}",
                register,
                step,
                value
            );
        });
    }

    // LOW-DEGREE EXTENSION
    // --------------------------------------------------------------------------------------------
    /// Extends all registers of the trace table to the length of the LDE domain; The extension
    /// is done by first interpolating a register into a polynomial and then evaluating the
    /// polynomial over the LDE domain.
    pub fn extend(mut self, domain: &StarkDomain) -> (TraceTable, PolyTable) {
        assert_eq!(
            self.len(),
            domain.trace_length(),
            "inconsistent trace length"
        );
        // build and cache trace twiddles for FFT interpolation; we do it here so that we
        // don't have to rebuild these twiddles for every register.
        let inv_twiddles = fft::get_inv_twiddles::<BaseElement>(domain.trace_length());

        // extend all registers (either in multiple threads or in a single thread); the extension
        // procedure first interpolates register traces into polynomials (in-place), then evaluates
        // these polynomials over a larger domain, and then returns extended evaluations.
        #[cfg(feature = "concurrent")]
        let extended_trace = self
            .0
            .par_iter_mut()
            .map(|register_trace| extend_register(register_trace, &domain, &inv_twiddles))
            .collect();

        #[cfg(not(feature = "concurrent"))]
        let extended_trace = self
            .0
            .iter_mut()
            .map(|register_trace| extend_register(register_trace, &domain, &inv_twiddles))
            .collect();

        (
            TraceTable::new(extended_trace, domain.trace_to_lde_blowup()),
            PolyTable::new(self.0),
        )
    }
}

// HELPER FUNCTIONS
// ================================================================================================

#[inline(always)]
fn extend_register(
    trace: &mut [BaseElement],
    domain: &StarkDomain,
    inv_twiddles: &[BaseElement],
) -> Vec<BaseElement> {
    let domain_offset = domain.offset();
    let twiddles = domain.trace_twiddles();
    let blowup_factor = domain.trace_to_lde_blowup();

    // interpolate register trace into a polynomial; we do this over the un-shifted trace_domain
    fft::interpolate_poly(trace, inv_twiddles);

    // evaluate the polynomial over extended domain; the domain may be shifted by the
    // domain_offset
    fft::evaluate_poly_with_offset(trace, twiddles, domain_offset, blowup_factor)
}
