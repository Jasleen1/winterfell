use super::{StarkDomain, TracePolyTable, TraceTable};
use common::{utils::uninit_vector, Assertions};
use math::{
    fft,
    field::{BaseElement, FieldElement},
};

#[cfg(feature = "concurrent")]
use rayon::prelude::*;

// CONSTANTS
// ================================================================================================

const MIN_TRACE_LENGTH: usize = 8;
const MIN_FRAGMENT_LENGTH: usize = 2;

// TRACE TABLE
// ================================================================================================
pub struct ExecutionTrace(Vec<Vec<BaseElement>>);

impl ExecutionTrace {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new execution trace of the specified width and length; data in the trace is not
    /// initialized and it is expected that the trace will be filled using one of the data mutator
    /// methods.
    pub fn new(width: usize, length: usize) -> Self {
        assert!(
            width > 0,
            "execution trace must consist of at least one register"
        );
        assert!(
            length >= MIN_TRACE_LENGTH,
            "execution trace must be at lest {} steps long, but was {}",
            MIN_TRACE_LENGTH,
            length
        );
        assert!(
            length.is_power_of_two(),
            "execution trace length must be a power of 2"
        );

        let registers = (0..width).map(|_| uninit_vector(length)).collect();
        ExecutionTrace(registers)
    }

    /// Creates a new execution trace from a list of provided register traces.
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

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Fills all rows in the execution trace using the specified closures as follows:
    /// - `init` closure is used to initialize the first row of the trace; it receives a mutable
    ///   reference to the first state initialized to all zeros. The contents of the state are
    ///   copied into the first row of the trace after the closure returns.
    /// - `update` closure is used to populate all subsequent rows of the trace; it receives two
    ///   parameters:
    ///   - index of the last updated row (starting with 0).
    ///   - a mutable reference to the last updated state; the contents of the state are copied
    ///     into the next row of the trace after the closure returns.
    pub fn fill<I, U>(&mut self, init: I, update: U)
    where
        I: Fn(&mut [BaseElement]),
        U: Fn(usize, &mut [BaseElement]),
    {
        let mut state = vec![BaseElement::ZERO; self.width()];
        init(&mut state);
        self.update_row(0, &state);

        for i in 0..self.len() - 1 {
            update(i, &mut state);
            self.update_row(i + 1, &state);
        }
    }

    /// Updates a single row in the execution trace with provided data.
    pub fn update_row(&mut self, step: usize, state: &[BaseElement]) {
        for (register, &value) in self.0.iter_mut().zip(state) {
            register[step] = value;
        }
    }

    /// Breaks the execution trace into mutable fragments each having the number of rows
    /// specified by `fragment_length` parameter. The returned fragments can be used to
    /// update data in the trace from multiple threads.
    pub fn fragments(&mut self, fragment_length: usize) -> Vec<ExecutionTraceFragment> {
        assert!(
            fragment_length >= MIN_FRAGMENT_LENGTH,
            "fragment length must be at least {}, but was {}",
            MIN_FRAGMENT_LENGTH,
            fragment_length
        );
        assert!(
            fragment_length.is_power_of_two(),
            "fragment length must be a power of 2"
        );
        let num_fragments = self.len() / fragment_length;

        let mut fragment_data = (0..num_fragments).map(|_| Vec::new()).collect::<Vec<_>>();
        self.0.iter_mut().for_each(|column| {
            for (i, fragment) in column.chunks_mut(fragment_length).enumerate() {
                fragment_data[i].push(fragment);
            }
        });

        fragment_data
            .into_iter()
            .enumerate()
            .map(|(i, data)| ExecutionTraceFragment {
                offset: i * fragment_length,
                data,
            })
            .collect()
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
    pub fn extend(mut self, domain: &StarkDomain) -> (TraceTable, TracePolyTable) {
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
            TracePolyTable::new(self.0),
        )
    }
}

// TRACE FRAGMENTS
// ================================================================================================

pub struct ExecutionTraceFragment<'a> {
    offset: usize,
    data: Vec<&'a mut [BaseElement]>,
}

impl<'a> ExecutionTraceFragment<'a> {
    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------
    /// Returns the step at which the fragment starts.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the length of this execution trace fragment.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.data[0].len()
    }

    /// Returns the width of the fragment (same as the width of the underlying table)
    pub fn width(&self) -> usize {
        self.data.len()
    }

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Fills all rows in the fragment using the specified closures as follows:
    /// - `init` closure is used to initialize the first row of the fragment; it receives a
    ///   mutable reference to the first state initialized to all zeros. Contents contents of the
    ///   state are copied into the first row of the fragment after the closure returns.
    /// - `update` closure is used to populate all subsequent rows of the fragment; it receives two
    ///   parameters:
    ///   - index of the last updated row (starting with 0).
    ///   - a mutable reference to the last updated state; the contents of the state are copied
    ///     into the next row of the fragment after the closure returns.
    pub fn fill<I, T>(&mut self, init_state: I, update_state: T)
    where
        I: Fn(&mut [BaseElement]),
        T: Fn(usize, &mut [BaseElement]),
    {
        let mut state = vec![BaseElement::ZERO; self.width()];
        init_state(&mut state);
        self.update_row(0, &state);

        for i in 0..self.len() - 1 {
            update_state(i, &mut state);
            self.update_row(i + 1, &state);
        }
    }

    /// Updates a single row in the fragment with provided data.
    pub fn update_row(&mut self, row_idx: usize, row_data: &[BaseElement]) {
        for (column, &value) in self.data.iter_mut().zip(row_data) {
            column[row_idx] = value;
        }
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
