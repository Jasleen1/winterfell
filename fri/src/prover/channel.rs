use crate::{FriOptions, PublicCoin};
use crypto::HashFunction;
use std::{convert::TryInto, mem::size_of};

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
    domain_size: usize,
    num_queries: usize,
}

impl DefaultProverChannel {
    pub fn new(options: FriOptions, domain_size: usize, num_queries: usize) -> Self {
        DefaultProverChannel {
            commitments: Vec::new(),
            options,
            domain_size,
            num_queries,
        }
    }

    pub fn draw_query_positions(&self) -> Vec<usize> {
        let hash = self.options.hash_fn();
        let lde_blowup_factor = self.options.blowup_factor();

        // determine how many bits are needed to represent valid indexes in the domain
        let value_mask = self.domain_size - 1;
        let value_offset = 32 - size_of::<usize>();

        // initialize the seed for PRNG
        let mut seed = [0u8; 64];
        seed[..32].copy_from_slice(&self.query_seed());
        let mut value_bytes = [0u8; 32];

        // draw values from PRNG until we get as many unique values as specified by
        // num_queries, but skipping values which are a multiple of blowup factor
        let mut result = Vec::new();
        for i in 0usize..1000 {
            // update the seed with the new counter and hash the result
            seed[56..].copy_from_slice(&i.to_le_bytes());
            hash(&seed, &mut value_bytes);

            // read the required number of bits from the hashed value
            let value =
                usize::from_le_bytes(value_bytes[value_offset..].try_into().unwrap()) & value_mask;

            if value % lde_blowup_factor == 0 {
                continue;
            }
            if result.contains(&value) {
                continue;
            }
            result.push(value);
            if result.len() >= self.num_queries {
                break;
            }
        }

        assert!(
            result.len() == self.num_queries,
            "needed to generate {} query positions, but generated only {}",
            self.num_queries,
            result.len()
        );

        result
    }

    fn query_seed(&self) -> [u8; 32] {
        // combine roots of all FIR layers into a single array of bytes
        let mut root_bytes: Vec<u8> = Vec::with_capacity(self.commitments.len() * 32);
        for root in self.commitments.iter() {
            root.iter().for_each(|&v| root_bytes.push(v));
        }

        // hash the array of bytes into a single 32-byte value
        let mut query_seed = [0u8; 32];
        self.options.hash_fn()(&root_bytes, &mut query_seed);

        query_seed
    }
}

impl ProverChannel for DefaultProverChannel {
    fn commit_fri_layer(&mut self, layer_root: [u8; 32]) {
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
