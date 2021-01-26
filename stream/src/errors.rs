use crate::ObjectId;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ObjectSendError {
    #[error("could not send objects: some objects are scheduled for deletion")]
    DeletionScheduled(Vec<ObjectId>),

    #[error("could not send objects")]
    Unknown(#[from] crate::Error),
}

#[derive(Error, Debug)]
pub enum ObjectReceiveError {
    #[error("could not receive objects: some objects are already being received")]
    AlreadyReceiving(Vec<ObjectId>),

    #[error("could not receive objects: some objects are already in the store")]
    AlreadyInStore(Vec<ObjectId>),

    #[error("could not receive objects: object metadata ({0} bytes) is too larger")]
    ObjectMetaTooLarge(usize),

    #[error("could not receive objects: object data ({0} bytes) is too larger")]
    ObjectDataTooLarge(usize),

    #[error("could not receive objects")]
    Unknown(#[from] crate::Error),
}
