use crate::{FriOptions, PublicCoin};
use crypto::HashFunction;

// PROVER CHANNEL TRAIT
// ================================================================================================

pub trait ProverChannel: PublicCoin {
    fn commit_fri_layer(&mut self, layer_root: [u8; 32]);
}

// DEFAULT PROVER CHANNEL IMPLEMENTATION
// ================================================================================================

pub struct DefaultProverChannel {
    commitments: Vec<[u8; 32]>,
    options: FriOptions,
}

impl DefaultProverChannel {
    pub fn new(options: FriOptions) -> Self {
        DefaultProverChannel {
            commitments: Vec::new(),
            options,
        }
    }

    pub fn commit_fri_layer(&mut self, layer_root: [u8; 32]) {
        self.commitments.push(layer_root);
    }
}

impl PublicCoin for DefaultProverChannel {
    fn fri_layer_commitments(&self) -> &[[u8; 32]] {
        &self.commitments
    }

    fn hash_fn(&self) -> HashFunction {
        self.options.hash_fn()
    }
}
