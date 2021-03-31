use crate::channel::ProverChannel;
use common::{
    errors::ProverError, proof::StarkProof, Assertions, ComputationContext, FieldExtension,
    ProofOptions, TransitionEvaluator,
};
use math::field::{BaseElement, QuadElement};
use std::marker::PhantomData;

mod domain;
use domain::StarkDomain;

mod constraints;
mod deep_fri;

mod trace;
pub use trace::{ExecutionTrace, ExecutionTraceFragment, TracePolyTable};

mod generation;
use generation::generate_proof;

#[cfg(test)]
mod tests;

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
        trace: ExecutionTrace,
        assertions: Assertions,
    ) -> Result<StarkProof, ProverError> {
        // make sure the assertions are valid
        trace.validate_assertions(&assertions);

        // create context to hold basic parameters for the computation; the context is also
        // used as a single source for such parameters as domain sizes, constraint degrees etc.
        let context = ComputationContext::new(
            trace.width(),
            trace.len(),
            T::get_ce_blowup_factor(),
            self.options.clone(),
        );

        match self.options.field_extension() {
            FieldExtension::None => generate_proof::<T, BaseElement>(trace, assertions, context),
            FieldExtension::Quadratic => {
                generate_proof::<T, QuadElement>(trace, assertions, context)
            }
        }
    }
}
