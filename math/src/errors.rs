use thiserror::Error;

#[derive(Error, Debug)]
pub enum SerializationError {
    #[error("destination must be at least {0} elements long, but was {1}")]
    DestinationTooSmall(usize, usize),

    #[error("failed to read element from bytes at position {0}")]
    FailedToReadElement(usize),

    #[error("number of bytes ({0}) does not divide into whole number of elements")]
    NotEnoughBytesForWholeElements(usize),
}
