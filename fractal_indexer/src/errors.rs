//! A list of error types which are produced during an execution of the protocol

use displaydoc::Display;
use thiserror::Error;

/// Represents a generic error type
#[derive(Debug, Display, Error)]
pub enum IndexerError {
    /// Error produced by the prover
    R1CS(R1CSError),
}

impl From<R1CSError> for IndexerError {
    fn from(e: R1CSError) -> IndexerError {
        IndexerError::R1CS(e)
    }
}

/// Represents errors in instantiating R1CS types 
#[derive(Debug, Display, Error)]
pub enum R1CSError {
    /// Matrix should consist of a vector of equal length vectors. Not the case here.
    InvalidMatrix(String),
    /// All matrices in R1CS should have equal dimensions
    MatrixSizeMismatch(String, String),
}


