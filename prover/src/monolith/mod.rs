use common::{
    stark::{
        Assertion, AssertionEvaluator, ConstraintEvaluator, ProofContext, ProofOptions, PublicCoin,
        StarkProof, TraceInfo, TransitionEvaluator,
    },
    utils::log2,
};
use log::debug;
use std::marker::PhantomData;
use std::time::Instant;

mod types;
use types::{CompositionPoly, TraceTable};

mod trace;
use trace::{build_lde_domain, commit_trace, extend_trace, query_trace};

mod constraints;
use constraints::{
    build_constraint_poly, commit_constraints, evaluate_constraints, extend_constraint_evaluations,
    query_constraints,
};

mod deep_fri;
use deep_fri::{compose_constraint_poly, compose_trace_polys, evaluate_composition_poly, fri};

use crate::channel::ProverChannel;

mod utils;

#[cfg(test)]
mod tests;

// PROVER
// ================================================================================================

pub struct Prover<T: TransitionEvaluator, A: AssertionEvaluator> {
    options: ProofOptions,
    _marker1: PhantomData<T>,
    _marker2: PhantomData<A>,
}

impl<T: TransitionEvaluator, A: AssertionEvaluator> Prover<T, A> {
    // Creates a new prover for the specified `options`. Generic parameters T and A
    // define specifics of the computation for this prover.
    // TODO: set a default value for A?
    pub fn new(options: ProofOptions) -> Prover<T, A> {
        Prover {
            options,
            _marker1: PhantomData,
            _marker2: PhantomData,
        }
    }

    /// Generates a STARK proof attesting that the `trace` satisfies the `assertions` and that
    // it is valid in the context of the computation described by this prover.
    pub fn prove(&self, trace: Vec<Vec<u128>>, assertions: Vec<Assertion>) -> StarkProof {
        let trace = TraceTable::new(trace);
        validate_assertions(&trace, &assertions);

        // TODO: get max_constraint_degree from TransitionEvaluator
        let context = ProofContext::new(
            trace.num_registers(),
            trace.num_states(),
            1,
            self.options.clone(),
        );

        let mut channel = ProverChannel::new(&context);

        // save trace info here, before trace table is extended
        let trace_info = TraceInfo::new(
            trace.num_registers(),
            trace.num_states(),
            self.options.blowup_factor(),
        );

        // 1 ----- extend execution trace -------------------------------------------------------------

        // build LDE domain; this is used later for polynomial evaluations
        let now = Instant::now();
        let lde_domain = build_lde_domain(&context);
        debug!(
            "Built LDE domain of 2^{} elements in {} ms",
            log2(lde_domain.size()),
            now.elapsed().as_millis()
        );

        // extend the execution trace; this interpolates each register of the trace into a polynomial,
        // and then evaluates the polynomial over LDE domain; each of the trace polynomials has
        // degree = trace_length - 1
        let (extended_trace, trace_polys) = extend_trace(trace, &lde_domain);
        debug!(
            "Extended execution trace of {} registers from 2^{} to 2^{} steps ({}x blowup) in {} ms",
            extended_trace.num_registers(),
            log2(trace_polys.poly_size()),
            log2(extended_trace.num_states()),
            context.options().blowup_factor(),
            now.elapsed().as_millis()
        );

        // 2 ----- commit to the extended execution trace -----------------------------------------
        let now = Instant::now();
        let trace_tree = commit_trace(&extended_trace, self.options.hash_fn());
        channel.commit_trace(*trace_tree.root());
        debug!(
            "Committed to extended execution trace by building a Merkle tree of depth {} in {} ms",
            trace_tree.depth(),
            now.elapsed().as_millis()
        );

        // 3 ----- evaluate constraints -----------------------------------------------------------
        let now = Instant::now();

        // build constraint evaluator using root of the trace Merkle tree as a seed to draw
        // random values; these values are used by the evaluator to compute a random linear
        // combination of constraint evaluations
        let evaluator = ConstraintEvaluator::<T, A>::new(&channel, &trace_info, assertions);

        // apply constraint evaluator to the extended trace table to generate a
        // constraint evaluation table
        let constraint_evaluations = evaluate_constraints(&evaluator, &extended_trace, &lde_domain);
        debug!(
            "Evaluated constraints over domain of 2^{} elements in {} ms",
            log2(constraint_evaluations.domain_size()),
            now.elapsed().as_millis()
        );

        // 4 ----- commit to constraint evaluations -----------------------------------------------

        // first, build a single constraint polynomial from all constraint evaluations
        let now = Instant::now();
        let constraint_poly = build_constraint_poly(constraint_evaluations, &context);
        debug!(
            "Converted constraint evaluations into a single polynomial of degree {} in {} ms",
            constraint_poly.degree(),
            now.elapsed().as_millis()
        );

        // then, evaluate constraint polynomial over LDE domain
        let now = Instant::now();
        let combined_constraint_evaluations =
            extend_constraint_evaluations(&constraint_poly, &lde_domain);
        debug!(
            "Evaluated constraint polynomial over LDE domain (2^{} elements) in {} ms",
            log2(combined_constraint_evaluations.len()),
            now.elapsed().as_millis()
        );

        // finally, commit to constraint polynomial evaluations
        let now = Instant::now();
        let constraint_tree =
            commit_constraints(combined_constraint_evaluations, self.options.hash_fn());
        channel.commit_constraints(*constraint_tree.root());
        debug!(
            "Committed to constraint evaluations by building a Merkle tree of depth {} in {} ms",
            constraint_tree.depth(),
            now.elapsed().as_millis()
        );

        // 5 ----- build DEEP composition polynomial ----------------------------------------------
        let now = Instant::now();

        // draw an out-of-domain point z from the entire field,
        let z = channel.draw_z();

        // allocate memory for the composition polynomial
        let mut composition_poly =
            CompositionPoly::new(context.lde_domain_size(), context.deep_composition_degree());

        // draw random coefficients to use during polynomial composition
        let coefficients = channel.draw_composition_coefficients();

        // combine all trace polynomials together and merge them into the composition polynomial;
        // deep_values are trace states at two out-of-domain points, and will go into the proof
        let deep_values = compose_trace_polys(&mut composition_poly, trace_polys, z, &coefficients);

        // merge constraint polynomial into the composition polynomial
        compose_constraint_poly(&mut composition_poly, constraint_poly, z, &coefficients);

        debug!(
            "Built DEEP composition polynomial of degree {} in {} ms",
            composition_poly.degree(),
            now.elapsed().as_millis()
        );

        // 6 ----- evaluate DEEP composition polynomial over LDE domain ---------------------------
        let now = Instant::now();
        let composed_evaluations = evaluate_composition_poly(composition_poly, &lde_domain);
        debug_assert_eq!(
            evaluator.deep_composition_degree(),
            utils::infer_degree(&composed_evaluations)
        );
        debug!(
            "Evaluated DEEP composition polynomial over LDE domain (2^{} elements) in {} ms",
            log2(evaluator.lde_domain_size()),
            now.elapsed().as_millis()
        );

        // 7 ----- compute FRI layers for the composition polynomial ------------------------------
        let now = Instant::now();
        let (fri_trees, fri_values) =
            fri::reduce(&composed_evaluations, &lde_domain, &self.options);
        debug!(
            "Computed {} FRI layers from composition polynomial evaluations in {} ms",
            fri_trees.len(),
            now.elapsed().as_millis()
        );

        // 8 ----- determine query positions ------------------------------------------------------
        let now = Instant::now();

        // combine all FRI layer roots into a single vector
        let mut fri_roots: Vec<u8> = Vec::new();
        for tree in fri_trees.iter() {
            tree.root().iter().for_each(|&v| fri_roots.push(v));
        }

        // commit to FRI layers
        let pow_nonce = channel.commit_fri(fri_roots);

        // generate pseudo-random query positions
        let positions = channel.draw_query_positions();
        debug!(
            "Determined {} query positions in {} ms",
            positions.len(),
            now.elapsed().as_millis()
        );

        // 9 ----- build proof object -------------------------------------------------------------
        let now = Instant::now();

        // generate FRI proof
        let fri_proof = fri::build_proof(fri_trees, fri_values, &positions);

        // query the execution trace at the selected position; for each query, we need the
        // state of the trace at that position + Merkle authentication path from trace_root
        let (trace_root, trace_proof, trace_states) =
            query_trace(extended_trace, trace_tree, &positions);

        // query the constraint evaluations at the selected positions; for each query, we
        // need just a Merkle authentication path from constraint_root. this is because
        // constraint evaluations for each step are merged into a single value and Merkle
        // authentication paths contain these values already
        let (constraint_root, constraint_proof) = query_constraints(constraint_tree, &positions);

        // build the proof object
        let proof = StarkProof::new(
            trace_root,
            trace_proof,
            trace_states,
            constraint_root,
            constraint_proof,
            evaluator.max_constraint_degree(),
            deep_values,
            fri_proof,
            pow_nonce,
            self.options.clone(),
        );
        debug!("Built proof object in {} ms", now.elapsed().as_millis());

        proof
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn validate_assertions(trace: &TraceTable, assertions: &[Assertion]) {
    // TODO: check for duplicated assertions
    // TODO: eventually, this should return errors instead of panicking
    assert!(
        !assertions.is_empty(),
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
