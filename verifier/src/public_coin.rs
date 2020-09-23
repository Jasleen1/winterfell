use common::stark::{Commitments, FriProof, ProofContext, PublicCoin};
use crypto::HashFunction;

pub struct VerifierCoin {
    context: ProofContext,
    constraint_seed: [u8; 32],
    composition_seed: [u8; 32],
    query_seed: [u8; 32],
}

impl VerifierCoin {
    pub fn new(context: &ProofContext, commitments: &Commitments, fri_proof: &FriProof) -> Self {
        let hash_fn = context.options().hash_fn();
        VerifierCoin {
            context: context.clone(),
            constraint_seed: commitments.trace_root,
            composition_seed: commitments.constraint_root,
            query_seed: build_query_seed(fri_proof, hash_fn),
        }
    }
}

impl PublicCoin for VerifierCoin {
    fn context(&self) -> &ProofContext {
        &self.context
    }

    fn constraint_seed(&self) -> [u8; 32] {
        self.constraint_seed
    }

    fn composition_seed(&self) -> [u8; 32] {
        self.composition_seed
    }

    fn query_seed(&self) -> [u8; 32] {
        self.query_seed
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_query_seed(fri_proof: &FriProof, hash_fn: HashFunction) -> [u8; 32] {
    // combine roots of all FIR layers into a single array of bytes
    let mut fri_roots: Vec<u8> = Vec::new();
    for layer in fri_proof.layers.iter() {
        layer.root.iter().for_each(|&v| fri_roots.push(v));
    }
    fri_proof.rem_root.iter().for_each(|&v| fri_roots.push(v));

    // hash the array of bytes into a single 32-byte value
    let mut query_seed = [0u8; 32];
    hash_fn(&fri_roots, &mut query_seed);

    /*
    // TODO
    let seed = match utils::verify_pow_nonce(seed, proof.pow_nonce(), &self.options) {
        Ok(seed) => seed,
        Err(msg) => return Err(msg)
    };
    */

    query_seed
}
