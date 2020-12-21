#![cfg_attr(nightly, feature(unsized_variants))] // This is required by https://github.com/kompics/kompact/blob/b3d31ba48abd1a131c3c1815b8f1a74c07a5f804/core/src/utils.rs#L608

pub mod hash;
pub mod lamport;

pub mod merkle;
pub use merkle::{build_merkle_nodes, BatchMerkleProof, MerkleTree};

pub type HashFunction = fn(&[u8], &mut [u8]);

pub mod utils;

mod random;
pub use random::RandomElementGenerator;
