use crate::channel::ProverChannel;
use common::{
    errors::ProverError, proof::StarkProof, Assertions, ComputationContext, FieldExtension,
    ProofOptions, TransitionEvaluator,
};
use math::field::{BaseElement, QuadExtension};
use std::marker::PhantomData;

mod types;
use types::{CompositionPoly, ConstraintEvaluationTable, TraceTable};

mod constraints;
mod deep_fri;
mod trace;

mod utils;

mod generation;
use generation::generate_proof;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================
#[cfg(not(feature = "extension_field"))]
const FIELD_EXTENSION: FieldExtension = FieldExtension::None;
#[cfg(feature = "extension_field")]
const FIELD_EXTENSION: FieldExtension = FieldExtension::Quadratic;

// PROVER
// ================================================================================================

pub struct Prover<T: TransitionEvaluator> {
    options: ProofOptions,
    _marker1: PhantomData<T>,
}

impl<T: TransitionEvaluator> Prover<T> {
    /// Creates a new prover for the specified `options`. Generic parameters T and A
    /// define specifics of the computation for this prover.
    pub fn new(options: ProofOptions) -> Prover<T> {
        Prover {
            options,
            _marker1: PhantomData,
        }
    }

    /// Generates a STARK proof attesting that the `trace` satisfies the `assertions` and that
    /// it is valid in the context of the computation described by this prover.
    pub fn prove(
        &self,
        trace: Vec<Vec<BaseElement>>,
        assertions: Assertions,
    ) -> Result<StarkProof, ProverError> {
        let trace = TraceTable::new(trace);
        validate_assertions(&trace, &assertions);

        // create context to hold basic parameters for the computation; the context is also
        // used as a single source for such parameters as domain sizes, constraint degrees etc.
        let context = ComputationContext::new(
            trace.num_registers(),
            trace.num_states(),
            T::get_ce_blowup_factor(),
            FIELD_EXTENSION,
            self.options.clone(),
        );

        match context.field_extension() {
            FieldExtension::None => generate_proof::<T, BaseElement>(trace, assertions, context),
            FieldExtension::Quadratic => {
                generate_proof::<T, QuadExtension<BaseElement>>(trace, assertions, context)
            }
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn validate_assertions(trace: &TraceTable, assertions: &Assertions) {
    // TODO: eventually, this should return errors instead of panicking
    assert!(
        !assertions.is_empty(),
        "at least one assertion must be provided"
    );

    assertions.for_each(|register, step, value| {
        assert!(
            value == trace.get(register, step),
            "trace does not satisfy assertion trace({}, {}) == {}",
            register,
            step,
            value
        );
    });
}
