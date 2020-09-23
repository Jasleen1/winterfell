use common::stark::{
    Assertion, AssertionEvaluator, ConstraintEvaluator, ProofContext, ProofOptions, PublicCoin,
    StarkProof, TransitionEvaluator,
};
use crypto::MerkleTree;
use math::field;

use std::marker::PhantomData;

mod composition;
use composition::{compose_constraints, compose_registers};

mod constraints;
use constraints::evaluate_constraints;

mod fri;

mod public_coin;
use public_coin::VerifierCoin;

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
        let hash_fn = self.options.hash_fn();

        // build context and public coin
        // TODO: move into proof implementation
        let context = ProofContext::new(
            proof.trace_info().width(),
            proof.trace_info().length(),
            proof.max_constraint_degree(),
            proof.options().clone(),
        );
        let coin = VerifierCoin::new(
            &context,
            *proof.trace_root(),
            *proof.constraint_root(),
            proof.fri_proof(),
        );

        // 1 ----- Verify proof of work and determine query positions -----------------------------

        let query_positions = coin.draw_query_positions();
        let c_positions = map_trace_to_constraint_positions(&query_positions);

        // 2 ----- Verify trace and constraint Merkle proofs --------------------------------------
        let trace_root = *proof.trace_root();
        let trace_proof = proof.trace_proof();
        if !MerkleTree::verify_batch(&trace_root, &query_positions, &trace_proof, hash_fn) {
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

        // draw a pseudo-random out-of-domain point z
        let z = coin.draw_z();

        // build constraint evaluator
        let evaluator = ConstraintEvaluator::<T, A>::new(&coin, &context, assertions);

        // evaluate constraints at z
        let constraint_evaluation_at_z = evaluate_constraints(
            evaluator,
            proof.get_state_at_z1(),
            proof.get_state_at_z2(),
            z,
        );

        // 4 ----- Compute composition polynomial evaluations -------------------------------------

        let coefficients = coin.draw_composition_coefficients();

        // compute composition values separately for trace and constraints, and then add them together
        let t_composition = compose_registers(&proof, &context, &query_positions, z, &coefficients);
        let c_composition = compose_constraints(
            &proof,
            &query_positions,
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
        match fri::verify(
            &proof.fri_proof(),
            &evaluations,
            &query_positions,
            context.deep_composition_degree(),
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
