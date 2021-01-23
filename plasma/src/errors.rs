use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlasmaError {
    #[error("failed to connect to Plasma Store: {0}")]
    ConnectError(String),
    #[error("the object already exists in the Plasma Store")]
    AlreadyExists,
    #[error("the object has already been sealed")]
    AlreadySealed,
    #[error("the object is not mutable")]
    NotMutable,
    #[error("unknown error: {0}")]
    UnknownError(String),
}
