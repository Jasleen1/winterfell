use common::stark::{
    compute_trace_query_positions, draw_z_and_coefficients, Assertion, AssertionEvaluator,
    ConstraintEvaluator, ProofOptions, StarkProof, TransitionEvaluator,
};
use crypto::MerkleTree;
use math::field;

use std::marker::PhantomData;

mod composition;
use composition::{compose_constraints, compose_registers};

mod constraints;
use constraints::evaluate_constraints;

mod fri;

// VERIFIER
// ================================================================================================

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

    pub fn verify(&self, proof: StarkProof, assertions: Vec<Assertion>) -> Result<bool, String> {
        let trace_info = proof.trace_info();
        let hash_fn = self.options.hash_fn();

        // 1 ----- Verify proof of work and determine query positions -----------------------------
        let fri_proof = proof.fri_proof();
        let mut fri_roots: Vec<u8> = Vec::new();
        for layer in fri_proof.layers.iter() {
            layer.root.iter().for_each(|&v| fri_roots.push(v));
        }
        fri_proof.rem_root.iter().for_each(|&v| fri_roots.push(v));

        let mut seed = [0u8; 32];
        hash_fn(&fri_roots, &mut seed);
        /*
        // TODO
        let seed = match utils::verify_pow_nonce(seed, proof.pow_nonce(), &self.options) {
            Ok(seed) => seed,
            Err(msg) => return Err(msg)
        };
        */

        let t_positions =
            compute_trace_query_positions(seed, trace_info.lde_domain_size(), &self.options);
        let c_positions = map_trace_to_constraint_positions(&t_positions);

        // 2 ----- Verify trace and constraint Merkle proofs --------------------------------------
        let trace_root = *proof.trace_root();
        let trace_proof = proof.trace_proof();
        if !MerkleTree::verify_batch(&trace_root, &t_positions, &trace_proof, hash_fn) {
            return Err(String::from("verification of trace Merkle proof failed"));
        }

        let constraint_root = *proof.constraint_root();
        let constraint_proof = proof.constraint_proof();
        if !MerkleTree::verify_batch(&constraint_root, &c_positions, &constraint_proof, hash_fn) {
            return Err(String::from(
                "verification of constraint Merkle proof failed",
            ));
        }

        // 3 ----- Compute constraint evaluations at OOD point z ----------------------------------

        // derive OOD point z and composition coefficients from the root of the constraint tree
        // TODO: separate drawing of z and building of coefficients?
        let (z, coefficients) = draw_z_and_coefficients(constraint_root, trace_info.width());

        let evaluator = ConstraintEvaluator::<T, A>::new(trace_root, &trace_info, assertions);

        // evaluate constraints at z
        let constraint_evaluation_at_z = evaluate_constraints(
            evaluator,
            proof.get_state_at_z1(),
            proof.get_state_at_z2(),
            z,
        );

        // 4 ----- Compute composition polynomial evaluations -------------------------------------

        // compute composition values separately for trace and constraints, and then add them together
        let t_composition = compose_registers(&proof, &t_positions, z, &coefficients);
        let c_composition = compose_constraints(
            &proof,
            &t_positions,
            &c_positions,
            z,
            constraint_evaluation_at_z,
            &coefficients,
        );
        let evaluations = t_composition
            .iter()
            .zip(c_composition)
            .map(|(&t, c)| field::add(t, c))
            .collect::<Vec<u128>>();

        // 5 ----- Verify low-degree proof -------------------------------------------------------------
        let max_degree = get_composition_degree(trace_info.length(), proof.max_constraint_degree());

        match fri::verify(
            &fri_proof,
            &evaluations,
            &t_positions,
            max_degree,
            &self.options,
        ) {
            Ok(result) => Ok(result),
            Err(msg) => Err(format!("verification of low-degree proof failed: {}", msg)),
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================
pub fn map_trace_to_constraint_positions(positions: &[usize]) -> Vec<usize> {
    let mut result = Vec::with_capacity(positions.len());
    for &position in positions.iter() {
        let cp = position / 2;
        if !result.contains(&cp) {
            result.push(cp);
        }
    }
    result
}

// TODO: same as in prover. consolidate.
fn get_composition_degree(trace_length: usize, max_constraint_degree: usize) -> usize {
    std::cmp::max(max_constraint_degree - 1, 1) * trace_length - 1
}
