#![feature(iterator_fold_self)]

pub mod hash;
pub mod lamport;

pub mod merkle;
mod utils;

pub use merkle::{build_merkle_nodes, BatchMerkleProof, MerkleTree};

pub type HashFunction = fn(&[u8], &mut [u8]);

pub mod utils;
