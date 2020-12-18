use common::{
    errors::VerifierError, proof::StarkProof, Assertion, AssertionEvaluator, ComputationContext,
    DefaultAssertionEvaluator, FieldExtension, TransitionEvaluator,
};
use math::field::{BaseElement, QuadExtension};
use std::marker::PhantomData;

mod channel;
use channel::VerifierChannel;

mod verification;
use verification::perform_verification;

mod fri;

// VERIFIER
// ================================================================================================

pub struct Verifier<T: TransitionEvaluator, A: AssertionEvaluator = DefaultAssertionEvaluator> {
    _marker1: PhantomData<T>,
    _marker2: PhantomData<A>,
}

#[allow(clippy::new_without_default)]
impl<T: TransitionEvaluator, A: AssertionEvaluator> Verifier<T, A> {
    /// Creates a new verifier for a computation defined by generic parameters T and A.
    pub fn new() -> Verifier<T, A> {
        Verifier {
            _marker1: PhantomData,
            _marker2: PhantomData,
        }
    }

    /// Verifies the STARK `proof` attesting the assertions are valid in the context of
    /// the computation described by the verifier.
    pub fn verify(
        &self,
        proof: StarkProof,
        assertions: Vec<Assertion>,
    ) -> Result<bool, VerifierError> {
        // build the computation context from the proof. The context contains basic parameters
        // such as trace length, domain sizes, etc. It also defines whether extension field
        // should be used during verification.
        let context = build_context(&proof)?;

        // initializes a channel and perform verification procedure in the field defined in the
        // context; the channel is used to simulate interaction between the prover and the
        // verifier; the verifier can read the values written by the prover into the channel,
        // and also draws random values which the prover uses during proof construction
        match context.field_extension() {
            FieldExtension::None => {
                let channel = VerifierChannel::new(context, proof)?;
                perform_verification::<T, A, BaseElement>(&channel, assertions)
            }
            FieldExtension::Quadratic => {
                let channel = VerifierChannel::new(context, proof)?;
                perform_verification::<T, A, QuadExtension<BaseElement>>(&channel, assertions)
            }
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_context(proof: &StarkProof) -> Result<ComputationContext, VerifierError> {
    let options = proof.context.options.clone();
    let trace_length =
        usize::pow(2, proof.context.lde_domain_depth as u32) / options.blowup_factor();
    let field_extension = match proof.context.field_extension_factor {
        1 => FieldExtension::None,
        2 => FieldExtension::Quadratic,
        _ => return Err(VerifierError::ComputationContextDeserializationFailed),
    };
    // TODO: read modulus from the proof and check it against field modulus of base elements

    Ok(ComputationContext::new(
        proof.context.trace_width as usize,
        trace_length,
        proof.context.ce_blowup_factor as usize,
        field_extension,
        options,
    ))
}
