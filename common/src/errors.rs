//! A list of error types which are produced during an execution of the protocol

use crate::Assertion;
use displaydoc::Display;
use thiserror::Error;

/// Represents a generic error type
#[derive(Debug, Display, Error)]
pub enum ProtocolError {
    /// Error produced by the prover
    Prove(ProverError),
}

impl From<ProverError> for ProtocolError {
    fn from(e: ProverError) -> ProtocolError {
        ProtocolError::Prove(e)
    }
}

/// Represents an error thrown by the prover during an execution of the protocol
#[derive(Debug, Display, Error)]
pub enum ProverError {
    /// An error produced in evaluation
    Eval(EvaluatorError),
    /// A transition constraint was not satisfied at a certain step {0}
    UnsatisfiedTransitionConstraintError(usize),
    /// The constraint polynomial's components do not all have the same degree, expected {0} but found {1}
    MismatchedConstraintPolynomialDegree(usize, usize),
}

/// Represents an error thrown by the verifier during an execution of the protocol
#[derive(Debug, Display, Error)]
pub enum VerifierError {
    /// An error produced in evaluation
    Eval(EvaluatorError),
    /// Verification of low-degree proof failed: {0}
    FriVerificationFailed(fri::VerifierError),
    /// Trace query did not match the commitment
    TraceQueryDoesNotMatchCommitment,
    /// Trace query deserialization failed
    TraceQueryDeserializationFailed,
    /// Constraint query did not match the commitment
    ConstraintQueryDoesNotMatchCommitment,
    /// Query seed proof-of-work verification failed
    QuerySeedProofOfWorkVerificationFailed,
    /// Out-of-domain frame deserialization failed
    OodFrameDeserializationFailed,
    /// Computation context deserialization failed
    ComputationContextDeserializationFailed,
}

/// Represents an error thrown during evaluation
#[derive(Debug, Display, Error, PartialEq)]
pub enum AssertionError {
    /// Execution trace must be at least one register wide
    TraceWidthTooShort,
    /// Execution trace length ({0}) is not a power of two
    TraceLengthNotPowerOfTwo(usize),
    /// Duplicate assertion: {0}
    DuplicateAssertion(Assertion),
    /// Invalid register index {0}
    InvalidAssertionRegisterIndex(usize),
    /// Assertion trace length ({0}) is invalid; expected {1}
    InvalidAssertionTraceLength(usize, usize),
    /// Invalid assertion step {0}, must be in [0, {1})
    InvalidAssertionStep(usize, usize),
    /// Number of asserted values must be greater than zero
    ZeroAssertedValues,
    /// Number of asserted values ({0}) must be smaller than trace length ({1})
    TooManyAssertedValues(usize, usize),
    /// Number of asserted values ({0}) must be a power of two
    AssertedValuesNotPowerOfTwo(usize),
}

/// Represents an error thrown during evaluation
#[derive(Debug, Display, Error)]
pub enum EvaluatorError {
    /// At least one assertion must be provided
    NoAssertionsSpecified,
}

impl From<EvaluatorError> for ProverError {
    fn from(e: EvaluatorError) -> ProverError {
        ProverError::Eval(e)
    }
}

impl From<EvaluatorError> for VerifierError {
    fn from(e: EvaluatorError) -> VerifierError {
        VerifierError::Eval(e)
    }
}
