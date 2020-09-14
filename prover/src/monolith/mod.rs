use crate::{
    Assertion, AssertionEvaluator, ConstraintEvaluator, ProofOptions, TraceInfo,
    TransitionEvaluator,
};
use log::debug;
use std::marker::PhantomData;
use std::time::Instant;

mod types;
use types::TraceTable;

mod trace;
use trace::{build_lde_domain, commit_trace, extend_trace};

mod constraints;
use constraints::{
    build_constraint_poly, commit_constraints, evaluate_constraints, extend_constraint_evaluations,
};

mod fri;

// PROVER
// ================================================================================================

pub struct Prover<T: TransitionEvaluator, A: AssertionEvaluator> {
    options: ProofOptions,
    _marker1: PhantomData<T>,
    _marker2: PhantomData<A>,
}

impl<T: TransitionEvaluator, A: AssertionEvaluator> Prover<T, A> {
    pub fn new(options: ProofOptions) -> Prover<T, A> {
        Prover {
            options,
            _marker1: PhantomData,
            _marker2: PhantomData,
        }
    }

    /// Generates a STARK proof that the `trace` satisfies the `assertions` and that it is valid
    /// in the context of the computation described by this prover.
    /// TODO: return proof
    pub fn prove(&self, trace: Vec<Vec<u128>>, assertions: Vec<Assertion>) {
        let trace = TraceTable::new(trace);
        validate_assertions(&trace, &assertions);

        // save trace info here, before trace table is extended
        let trace_info = TraceInfo::new(
            trace.num_registers(),
            trace.num_states(),
            self.options.blowup_factor(),
        );

        // 1 ----- extend execution trace -------------------------------------------------------------

        // build LDE domain and LDE twiddles (for FFT evaluation over LDE domain)
        let now = Instant::now();
        let (lde_domain, lde_twiddles) = build_lde_domain(&trace_info);
        debug!(
            "Built LDE domain of {} elements in {} ms",
            lde_domain.len(),
            now.elapsed().as_millis()
        );

        // extend the trace table; this interpolates each register of the trace into a polynomial,
        // and then evaluates the polynomial over LDE domain
        let (trace, trace_polys) = extend_trace(trace, &lde_twiddles);
        debug!(
            "Extended execution trace of {} registers from {} to {} steps in {} ms",
            trace.num_registers(),
            trace_polys.poly_size(),
            trace.num_states(),
            now.elapsed().as_millis()
        );

        // 2 ----- commit to the extended execution trace -----------------------------------------
        let now = Instant::now();
        let trace_tree = commit_trace(&trace, self.options.hash_fn());
        debug!(
            "Committed to extended execution trace in {} ms",
            now.elapsed().as_millis()
        );

        // 3 ----- evaluate constraints -----------------------------------------------------------
        let now = Instant::now();
        let evaluator =
            ConstraintEvaluator::<T, A>::new(*trace_tree.root(), trace_info, &assertions);
        let constraint_evaluations = evaluate_constraints(&evaluator, &trace, &lde_domain);
        debug!(
            "Evaluated constraints over domain of {} elements in {} ms",
            constraint_evaluations.len(),
            now.elapsed().as_millis()
        );

        // 4 ----- commit to constraint evaluations -----------------------------------------------

        // first, build a single constraint polynomial from all constraint evaluations
        let now = Instant::now();
        let constraint_poly = build_constraint_poly(constraint_evaluations);
        debug!(
            "Converted constraint evaluations into a single polynomial of degree {} in {} ms",
            constraint_poly.len(), // TODO: degree(),
            now.elapsed().as_millis()
        );

        // then, evaluate constraint polynomial over LDE domain
        let now = Instant::now();
        let combined_constraint_evaluations =
            extend_constraint_evaluations(constraint_poly, &lde_twiddles);
        debug!(
            "Evaluated constraint polynomial over LDE domain in {} ms",
            now.elapsed().as_millis()
        );

        // finally, commit to constraint polynomial evaluations
        let now = Instant::now();
        let _constraint_tree =
            commit_constraints(&combined_constraint_evaluations, self.options.hash_fn());
        debug!(
            "Committed to constraint evaluations over LDE domain {} ms",
            now.elapsed().as_millis()
        );

        // 5 ----- build DEEP composition polynomial ----------------------------------------------

        // 6 ----- evaluate DEEP composition polynomial over LDE domain ---------------------------

        // 7 ----- compute FRI layers for the composition polynomial ------------------------------

        // 8 ----- determine query positions ------------------------------------------------------

        // 9 ----- build proof object -------------------------------------------------------------
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn validate_assertions(trace: &TraceTable, assertions: &[Assertion]) {
    // TODO: check for duplicated assertions
    // TODO: eventually, this should return errors instead of panicking
    assert!(
        assertions.len() > 0,
        "at least one assertion must be provided"
    );
    for assertion in assertions {
        assert!(
            trace.get(assertion.register(), assertion.step()) == assertion.value(),
            "trace does not satisfy assertion {}",
            assertion
        );
    }
}
