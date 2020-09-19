use common::stark::{
    compute_trace_query_positions, Assertion, AssertionEvaluator, ConstraintEvaluator,
    ProofOptions, StarkProof, TraceInfo, TransitionEvaluator,
};
use std::marker::PhantomData;

pub struct Verifier<T: TransitionEvaluator, A: AssertionEvaluator> {
    options: ProofOptions,
    _marker1: PhantomData<T>,
    _marker2: PhantomData<A>,
}

impl<T: TransitionEvaluator, A: AssertionEvaluator> Verifier<T, A> {
    pub fn new(options: ProofOptions) -> Verifier<T, A> {
        Verifier {
            options,
            _marker1: PhantomData,
            _marker2: PhantomData,
        }
    }

    pub fn verify(_proof: StarkProof, _assertions: Vec<Assertion>) -> Result<bool, String> {
        // TODO: implement
        Ok(true)
    }
}
