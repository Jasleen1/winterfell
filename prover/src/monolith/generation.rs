use common::{
    errors::ProverError, proof::StarkProof, utils::log2, Assertions, ComputationContext,
    ConstraintEvaluator, PublicCoin, TransitionEvaluator,
};
use log::debug;
use math::field::{BaseElement, FieldElement, FromVec};
use std::time::Instant;

use super::{
    constraints::{
        build_constraint_poly, build_constraint_tree, evaluate_constraints,
        extend_constraint_evaluations, query_constraints,
    },
    deep_fri::{compose_constraint_poly, compose_trace_polys, evaluate_composition_poly},
    trace::{build_trace_tree, extend_trace, query_trace},
    utils, CompositionPoly, ConstraintEvaluationTable, LdeDomain, ProverChannel, TraceTable,
};

// PROOF GENERATION PROCEDURE
// ================================================================================================

pub fn generate_proof<T, E>(
    trace: TraceTable,
    assertions: Assertions,
    context: ComputationContext,
) -> Result<StarkProof, ProverError>
where
    T: TransitionEvaluator,
    E: FieldElement + From<BaseElement> + FromVec<BaseElement>,
{
    // create a channel; this simulates interaction between the prover and the verifier;
    // the channel will be used to commit to values and to draw randomness that should
    // come from the verifier
    let mut channel = ProverChannel::new(&context);

    // 1 ----- extend execution trace -------------------------------------------------------------

    // build LDE domain; this is used later for polynomial evaluations
    let now = Instant::now();
    let lde_domain = LdeDomain::new(context.lde_domain_size());
    debug!(
        "Built LDE domain of 2^{} elements in {} ms",
        log2(lde_domain.size()),
        now.elapsed().as_millis()
    );

    // extend the execution trace; this interpolates each register of the trace into a polynomial,
    // and then evaluates the polynomial over the LDE domain; each of the trace polynomials has
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
    let trace_tree = build_trace_tree(&extended_trace, context.options().hash_fn());
    channel.commit_trace(*trace_tree.root());
    debug!(
        "Committed to extended execution trace by building a Merkle tree of depth {} in {} ms",
        trace_tree.depth(),
        now.elapsed().as_millis()
    );

    // 3 ----- evaluate constraints -----------------------------------------------------------
    let now = Instant::now();

    // build constraint evaluator; the channel is passed in for the evaluator to draw random
    // values from; these values are used by the evaluator to compute a random linear
    // combination of constraint evaluations
    let mut evaluator = ConstraintEvaluator::<T>::new(&channel, &context, assertions)?;

    // apply constraint evaluator to the extended trace table to generate a
    // constraint evaluation table
    let constraint_evaluations: ConstraintEvaluationTable<BaseElement> =
        evaluate_constraints(&mut evaluator, &extended_trace, &lde_domain)?;
    debug!(
        "Evaluated constraints over domain of 2^{} elements in {} ms",
        log2(constraint_evaluations.domain_size()),
        now.elapsed().as_millis()
    );

    // 4 ----- commit to constraint evaluations -----------------------------------------------

    // first, build a single constraint polynomial from all constraint evaluations
    let now = Instant::now();
    let constraint_poly = build_constraint_poly(constraint_evaluations, &context)?;
    debug!(
        "Converted constraint evaluations into a single polynomial of degree {} in {} ms",
        constraint_poly.degree(),
        now.elapsed().as_millis()
    );

    // then, evaluate constraint polynomial over the LDE domain
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
        build_constraint_tree(combined_constraint_evaluations, context.options().hash_fn());
    channel.commit_constraints(*constraint_tree.root());
    debug!(
        "Committed to constraint evaluations by building a Merkle tree of depth {} in {} ms",
        constraint_tree.depth(),
        now.elapsed().as_millis()
    );

    // 5 ----- build DEEP composition polynomial ----------------------------------------------
    let now = Instant::now();

    // draw an out-of-domain point z from the base field. If the extension_field feature flag is
    // enabled, then a random point from the extension field is sampled.
    //
    // The purpose of sampling from the extension field here (instead of the base field) is to
    // increase security. Soundness is limited by the size of the field that the random point
    // is drawn from, and we can potentially save on performance by only drawing this point
    // from an extension field, rather than increasing the size of the field overall.
    let z = channel.draw_deep_point::<E>();

    // allocate memory for the composition polynomial; this will allocate enough memory to
    // hold composition polynomial evaluations over the LDE domain (done in the next step)
    let mut composition_poly = CompositionPoly::new(&context);

    // draw random coefficients to use during polynomial composition
    let coefficients = channel.draw_composition_coefficients();

    // combine all trace polynomials together and merge them into the composition polynomial;
    // ood_frame are trace states at two out-of-domain points, and will go into the proof
    let ood_frame = compose_trace_polys(&mut composition_poly, trace_polys, z, &coefficients);

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
        context.deep_composition_degree(),
        utils::infer_degree(&composed_evaluations)
    );
    debug!(
        "Evaluated DEEP composition polynomial over LDE domain (2^{} elements) in {} ms",
        log2(context.lde_domain_size()),
        now.elapsed().as_millis()
    );

    // 7 ----- compute FRI layers for the composition polynomial ------------------------------
    let now = Instant::now();
    let mut fri_prover = fri::FriProver::new(context.options().to_fri_options());
    fri_prover.build_layers(&mut channel, composed_evaluations, &lde_domain.values());
    debug!(
        "Computed {} FRI layers from composition polynomial evaluations in {} ms",
        fri_prover.num_layers(),
        now.elapsed().as_millis()
    );

    // 8 ----- determine query positions ------------------------------------------------------
    let now = Instant::now();

    // apply proof-of-work to the query seed
    channel.grind_query_seed();

    // generate pseudo-random query positions
    let query_positions = channel.draw_query_positions();
    debug!(
        "Determined {} query positions in {} ms",
        query_positions.len(),
        now.elapsed().as_millis()
    );

    // 9 ----- build proof object -------------------------------------------------------------
    let now = Instant::now();

    // generate FRI proof
    let fri_proof = fri_prover.build_proof(&query_positions);

    // query the execution trace at the selected position; for each query, we need the
    // state of the trace at that position + Merkle authentication path
    let (trace_paths, trace_states) = query_trace(extended_trace, trace_tree, &query_positions);

    // query the constraint evaluations at the selected positions; for each query, we need just
    // a Merkle authentication path. this is because constraint evaluations for each step are
    // merged into a single value and Merkle authentication paths contain these values already
    let constraint_paths = query_constraints(constraint_tree, &query_positions);

    // build the proof object
    let proof = channel.build_proof(
        trace_paths,
        trace_states,
        constraint_paths,
        ood_frame,
        fri_proof,
    );
    debug!("Built proof object in {} ms", now.elapsed().as_millis());

    Ok(proof)
}
