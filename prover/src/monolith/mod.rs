use crate::{ConstraintEvaluator, ProofOptions};
use log::debug;
use std::marker::PhantomData;
use std::time::Instant;

mod types;
use types::{PolyTable, TraceTable};

mod trace;
use trace::{build_lde_domain, commit_trace, extend_trace};

mod constraints;
use constraints::evaluate_constraints;

pub struct Prover<E: ConstraintEvaluator> {
    options: ProofOptions,
    _marker: PhantomData<E>,
}

impl<E: ConstraintEvaluator> Prover<E> {
    pub fn new(options: ProofOptions) -> Prover<E> {
        Prover {
            options,
            _marker: PhantomData,
        }
    }

    pub fn prove(&self, trace: Vec<Vec<u128>>) {
        let trace = TraceTable::new(trace);
        let trace_length = trace.num_states();

        // 1 ----- extend execution trace -------------------------------------------------------------

        // build LDE domain and LDE twiddles (for FFT evaluation over LDE domain)
        let now = Instant::now();
        let (lde_domain, lde_twiddles) =
            build_lde_domain(trace.num_states(), self.options.blowup_factor());
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

        // 2 ----- Commit to the extended execution trace -----------------------------------------
        let now = Instant::now();
        let trace_tree = commit_trace(&trace, self.options.hash_fn());
        debug!(
            "Committed to extended execution trace in {} ms",
            now.elapsed().as_millis()
        );

        // 3 ----- Evaluate constraints -----------------------------------------------------------
        let _now = Instant::now();
        let evaluator = E::new(
            *trace_tree.root(),
            trace_length,
            self.options.blowup_factor(),
            vec![],
        );
        evaluate_constraints(&evaluator, &trace, &lde_domain);
        //constraints.evaluate(&trace, &lde_domain);
        //debug!(
        //  "Evaluated constraints over domain of {} elements in {} ms",
        //  10, // TODO
        //  now.elapsed().as_millis()
        //);

        // 4 ----- convert constraint evaluations into a polynomial -------------------------------
        let _now = Instant::now();
        //let constraint_poly = constraints.combine_polys();
        //debug!(
        //    "Converted constraint evaluations into a single polynomial of degree {} in {} ms",
        //    constraint_poly.len(), // TODO: degree(),
        //    now.elapsed().as_millis()
        //);

        // 5 ----- evaluate constraint polynomial over LDE domain ---------------------------------

        // 6 ----- commit to constraint polynomial evaluations ------------------------------------

        // 7 ----- build DEEP composition polynomial ----------------------------------------------

        // 8 ----- evaluate DEEP composition polynomial over LDE domain ---------------------------

        // 9 ----- compute FRI layers for the composition polynomial ------------------------------

        // 10 ---- determine query positions ------------------------------------------------------

        // 11 ---- build proof object -------------------------------------------------------------
    }
}
