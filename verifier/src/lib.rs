use common::stark::{
    compute_trace_query_positions, draw_z_and_coefficients, Assertion, AssertionEvaluator,
    ConstraintEvaluator, ProofOptions, StarkProof, TransitionEvaluator,
};
use crypto::MerkleTree;
use math::field;

use std::marker::PhantomData;

mod composition;
use composition::{compose_constraints, compose_registers};

mod fri;
mod quartic;

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

        //derive OOD point z and composition coefficients from the root of the constraint tree
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

        // 6 ----- Verify low-degree proof -------------------------------------------------------------
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

fn evaluate_constraints<T: TransitionEvaluator, A: AssertionEvaluator>(
    evaluator: ConstraintEvaluator<T, A>,
    state1: &[u128],
    state2: &[u128],
    x: u128,
) -> u128 {
    let (t_value, i_value, f_value) = evaluator.evaluate_at(state1, state2, x);

    // Z(x) = x - 1
    let z = field::sub(x, field::ONE);
    let mut result = field::div(i_value, z);

    // Z(x) = x - x_at_last_step
    let z = field::sub(x, get_x_at_last_step(evaluator.trace_length()));
    result = field::add(result, field::div(f_value, z));

    // Z(x) = (x^steps - 1) / (x - x_at_last_step)
    let z = field::div(
        field::sub(field::exp(x, evaluator.trace_length() as u128), field::ONE),
        z,
    );
    result = field::add(result, field::div(t_value, z));

    result
}

fn get_x_at_last_step(trace_length: usize) -> u128 {
    let trace_root = field::get_root_of_unity(trace_length);
    field::exp(trace_root, (trace_length - 1) as u128)
}
