use common::{errors::VerifierError, FieldExtension, ProofOptions};
use prover::{crypto::hash, Assertions, StarkProof};
use structopt::StructOpt;

pub mod anon;
pub mod fibonacci;
pub mod lamport;
pub mod merkle;
pub mod rescue;
pub mod utils;

#[cfg(test)]
mod tests;

// TYPES AND INTERFACES
// ================================================================================================

pub trait Example {
    fn prepare(&mut self, n: usize) -> Assertions;
    fn prove(&self, assertions: Assertions) -> StarkProof;
    fn verify(&self, proof: StarkProof, assertions: Assertions) -> Result<(), VerifierError>;
}

// EXAMPLE OPTIONS
// ================================================================================================

#[derive(StructOpt, Debug)]
#[structopt(name = "winterfell", about = "Winterfell examples")]
pub struct ExampleOptions {
    #[structopt(subcommand)]
    pub example: ExampleType,

    /// Number of queries included in a proof
    #[structopt(short = "q", long = "queries")]
    num_queries: Option<usize>,

    /// Blowup factor for low degree extension
    #[structopt(short = "b", long = "blowup")]
    blowup_factor: Option<usize>,

    /// Grinding factor for query seed
    #[structopt(short = "g", long = "grinding", default_value = "16")]
    grinding_factor: u32,

    /// Whether to use field extension for composition polynomial
    #[structopt(short = "e", long = "extension")]
    field_extension: bool,
}

impl ExampleOptions {
    pub fn to_proof_options(&self, q: usize, b: usize) -> ProofOptions {
        let num_queries = self.num_queries.unwrap_or(q);
        let blowup_factor = self.blowup_factor.unwrap_or(b);
        let field_extension = if self.field_extension {
            FieldExtension::Quadratic
        } else {
            FieldExtension::None
        };

        ProofOptions::new(
            num_queries,
            blowup_factor,
            self.grinding_factor,
            hash::blake3,
            field_extension,
        )
    }
}

#[derive(StructOpt, Debug)]
//#[structopt(about = "available examples")]
pub enum ExampleType {
    /// Compute a Fibonacci sequence using trace table with 2 registers
    Fib {
        /// Length of Fibonacci sequence; must be a power of two
        #[structopt(short = "n", default_value = "1048576")]
        sequence_length: usize,
    },
    /// Compute a Fibonacci sequence using trace table with 8 registers
    Fib8 {
        /// Length of Fibonacci sequence; must be a power of two
        #[structopt(short = "n", default_value = "1048576")]
        sequence_length: usize,
    },
    /// Compute a multiplicative Fibonacci sequence using trace table with 2 registers
    Mulfib {
        /// Length of Fibonacci sequence; must be a power of two
        #[structopt(short = "n", default_value = "1048576")]
        sequence_length: usize,
    },
    /// Compute a multiplicative Fibonacci sequence using trace table with 8 registers
    Mulfib8 {
        /// Length of Fibonacci sequence; must be a power of two
        #[structopt(short = "n", default_value = "1048576")]
        sequence_length: usize,
    },
    /// Compute a hash chain using Rescue hash function
    Rescue {
        /// Length of the hash chain; must be a power of two
        #[structopt(short = "n", default_value = "1024")]
        chain_length: usize,
    },
    /// Compute a root of a Merkle path using Rescue hash function
    Merkle {
        /// Depth of the Merkle tree; must be one less than a power of two
        #[structopt(short = "n", default_value = "7")]
        tree_depth: usize,
    },
    /// Compute public image of an anonymous token
    Anon {
        /// Depth of the Merkle tree; must be one less than a power of two
        #[structopt(short = "n", default_value = "7")]
        tree_depth: usize,
    },
    /// Compute an aggregate Lamport+ signature
    LamportA {
        /// Number of signatures to aggregate; must be a power of two
        #[structopt(short = "n", default_value = "4")]
        num_signatures: usize,
    },
    /// Compute a threshold Lamport+ signature
    LamportT {
        /// Number of signers; must be one less than a power of two
        #[structopt(short = "n", default_value = "3")]
        num_signers: usize,
    },
}
