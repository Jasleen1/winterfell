use super::VerifierCoin;
use common::{
    stark::{DeepValues, FriProof, ProofContext, StarkProof},
    utils::{as_bytes, uninit_vector},
};
use crypto::{BatchMerkleProof, MerkleTree};
use math::field;

pub trait StarkProofImpl {
    fn read_context(&self) -> ProofContext;
    fn init_public_coin(&self, context: &ProofContext) -> VerifierCoin;
    fn read_deep_values(&self) -> &DeepValues;
    fn read_trace_states(&self, positions: &[usize]) -> Result<Vec<Vec<u128>>, String>;
    fn read_constraint_evaluations(&self, positions: &[usize]) -> Result<Vec<u128>, String>;
    fn read_fri_proof(&self) -> &FriProof;
}

impl StarkProofImpl for StarkProof {
    fn read_context(&self) -> ProofContext {
        let context = &self.context;
        let trace_width = self.deep_values.trace_at_z1.len();
        let trace_length =
            usize::pow(2, context.lde_domain_depth as u32) / context.options.blowup_factor();

        ProofContext::new(
            trace_width,
            trace_length,
            context.max_constraint_degree as usize,
            context.options.clone(),
        )
    }

    fn init_public_coin(&self, context: &ProofContext) -> VerifierCoin {
        VerifierCoin::new(context, &self.commitments, &self.fri_proof)
    }

    fn read_deep_values(&self) -> &DeepValues {
        &self.deep_values
    }

    fn read_trace_states(&self, positions: &[usize]) -> Result<Vec<Vec<u128>>, String> {
        let queries = &self.queries;
        let hash_fn = self.context.options.hash_fn();

        let mut hashed_states = uninit_vector::<[u8; 32]>(queries.trace_states.len());
        #[allow(clippy::needless_range_loop)]
        for i in 0..queries.trace_states.len() {
            hash_fn(as_bytes(&queries.trace_states[i]), &mut hashed_states[i]);
        }

        let trace_proof = BatchMerkleProof {
            nodes: queries.trace_paths.clone(),
            values: hashed_states,
            depth: self.context.lde_domain_depth,
        };

        if !MerkleTree::verify_batch(
            &self.commitments.trace_root,
            positions,
            &trace_proof,
            hash_fn,
        ) {
            return Err(String::from("verification of trace Merkle proof failed"));
        }

        // TODO: get rid of this unnecessary copying
        let trace_states = queries
            .trace_states
            .iter()
            .map(|s| s.iter().copied().collect())
            .collect();

        Ok(trace_states)
    }

    fn read_constraint_evaluations(&self, positions: &[usize]) -> Result<Vec<u128>, String> {
        let queries = &self.queries;
        let hash_fn = self.context.options.hash_fn();

        let c_positions = map_trace_to_constraint_positions(positions);
        if !MerkleTree::verify_batch(
            &self.commitments.constraint_root,
            &c_positions,
            &queries.constraint_proof,
            hash_fn,
        ) {
            return Err(String::from(
                "verification of constraint Merkle proof failed",
            ));
        }

        // build constraint evaluation values from the leaves of constraint Merkle proof
        let mut evaluations: Vec<u128> = Vec::with_capacity(positions.len());
        let leaves = &queries.constraint_proof.values;
        for &position in positions.iter() {
            let leaf_idx = c_positions.iter().position(|&v| v == position / 2).unwrap();
            let element_start = (position % 2) * 16;
            let element_bytes = &leaves[leaf_idx][element_start..(element_start + 16)];
            evaluations.push(field::from_bytes(element_bytes));
        }

        Ok(evaluations)
    }

    fn read_fri_proof(&self) -> &FriProof {
        &self.fri_proof
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn map_trace_to_constraint_positions(positions: &[usize]) -> Vec<usize> {
    let mut result = Vec::with_capacity(positions.len());
    for &position in positions.iter() {
        let cp = position / 2;
        if !result.contains(&cp) {
            result.push(cp);
        }
    }
    result
}
