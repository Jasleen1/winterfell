use common::{
    stark::{
        compute_trace_query_positions, Assertion, AssertionEvaluator, ConstraintEvaluator,
        ProofOptions, StarkProof, TraceInfo, TransitionEvaluator,
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
use deep_fri::{
    compose_constraint_poly, compose_trace_polys, draw_z_and_coefficients,
    evaluate_composition_poly, fri,
};

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

        // save trace info here, before trace table is extended
        let trace_info = TraceInfo::new(
            trace.num_registers(),
            trace.num_states(),
            self.options.blowup_factor(),
        );

        // 1 ----- extend execution trace -------------------------------------------------------------

        // build LDE domain; this is used later for polynomial evaluations
        let now = Instant::now();
        let lde_domain = build_lde_domain(&trace_info);
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
            trace_info.blowup(),
            now.elapsed().as_millis()
        );

        // 2 ----- commit to the extended execution trace -----------------------------------------
        let now = Instant::now();
        let trace_tree = commit_trace(&extended_trace, self.options.hash_fn());
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
        let seed = *trace_tree.root();
        let evaluator = ConstraintEvaluator::<T, A>::new(seed, &trace_info, assertions);

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
        let constraint_poly = build_constraint_poly(constraint_evaluations, &trace_info);
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
        debug!(
            "Committed to constraint evaluations by building a Merkle tree of depth {} in {} ms",
            constraint_tree.depth(),
            now.elapsed().as_millis()
        );

        // 5 ----- build DEEP composition polynomial ----------------------------------------------
        let now = Instant::now();

        // use root of the constraint tree to draw an out-of-domain point z from the entire field,
        // and also draw random coefficients to use during polynomial composition
        let seed = *constraint_tree.root();
        let (z, coefficients) = draw_z_and_coefficients(seed, trace_info.width());

        // allocate memory for the composition polynomial
        let mut composition_poly = CompositionPoly::new(
            trace_info.lde_domain_size(),
            evaluator.deep_composition_degree(),
        );

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

        // derive a seed from the combined roots
        let mut seed = [0u8; 32];
        self.options.hash_fn()(&fri_roots, &mut seed);

        // apply proof-of-work to get a new seed
        let pow_nonce = 0;
        // TODO: let (seed, pow_nonce) = utils::find_pow_nonce(seed, &options);

        // generate pseudo-random query positions
        let positions = compute_trace_query_positions(seed, lde_domain.size(), &self.options);
        debug!(
            "Determined {} query positions from seed {} in {} ms",
            positions.len(),
            hex::encode(seed),
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
