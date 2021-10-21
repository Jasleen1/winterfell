//! A list of error types which are produced during an execution of the protocol

use crypto::MerkleTreeError;
use displaydoc::Display;
use thiserror::Error;

/// Represents a generic error type
#[derive(Debug, Display, Error)]
pub enum LincheckError {
    /// If the Merkle Tree leads to an error
    MerkleTreeErr(MerkleTreeError),
}

impl From<MerkleTreeError> for LincheckError {
    fn from(e: MerkleTreeError) -> LincheckError {
        LincheckError::MerkleTreeErr(e)
    }
}
