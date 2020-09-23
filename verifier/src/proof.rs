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
        let trace_width = self.trace_states[0].len();
        let trace_length =
            usize::pow(2, self.lde_domain_depth as u32) / self.options.blowup_factor();

        ProofContext::new(
            trace_width,
            trace_length,
            self.max_constraint_degree as usize,
            self.options.clone(),
        )
    }

    fn init_public_coin(&self, context: &ProofContext) -> VerifierCoin {
        VerifierCoin::new(
            context,
            self.trace_root,
            self.constraint_root,
            &self.fri_proof,
        )
    }

    fn read_deep_values(&self) -> &DeepValues {
        &self.deep_values
    }

    fn read_trace_states(&self, positions: &[usize]) -> Result<Vec<Vec<u128>>, String> {
        let hash_fn = self.options.hash_fn();

        let mut hashed_states = uninit_vector::<[u8; 32]>(self.trace_states.len());
        #[allow(clippy::needless_range_loop)]
        for i in 0..self.trace_states.len() {
            hash_fn(as_bytes(&self.trace_states[i]), &mut hashed_states[i]);
        }

        let trace_proof = BatchMerkleProof {
            nodes: self.trace_nodes.clone(),
            values: hashed_states,
            depth: self.lde_domain_depth,
        };

        if !MerkleTree::verify_batch(&self.trace_root, positions, &trace_proof, hash_fn) {
            return Err(String::from("verification of trace Merkle proof failed"));
        }

        // TODO: get rid of this unnecessary copying
        let trace_states = self
            .trace_states
            .iter()
            .map(|s| s.iter().copied().collect())
            .collect();

        Ok(trace_states)
    }

    fn read_constraint_evaluations(&self, positions: &[usize]) -> Result<Vec<u128>, String> {
        let hash_fn = self.options.hash_fn();
        let c_positions = map_trace_to_constraint_positions(positions);
        if !MerkleTree::verify_batch(
            &self.constraint_root,
            &c_positions,
            &self.constraint_proof,
            hash_fn,
        ) {
            return Err(String::from(
                "verification of constraint Merkle proof failed",
            ));
        }

        // build constraint evaluation values from the leaves of constraint Merkle proof
        let mut evaluations: Vec<u128> = Vec::with_capacity(positions.len());
        let leaves = &self.constraint_proof.values;
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
