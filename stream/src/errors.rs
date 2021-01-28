use crate::{status_codes, ObjectId};
use plasma::PlasmaError;
use std::{
    fmt::{self, Display, Formatter},
    net::SocketAddr,
};
use thiserror::{private::AsDynError, Error};

// OBJECT SEND ERROR
// ================================================================================================

#[derive(Debug)]
pub enum ObjectSendError {
    ObjectDeletionScheduled(SocketAddr, Vec<ObjectId>),
    ObjectMetaTooLarge(SocketAddr, ObjectId, usize),
    ObjectDataTooLarge(SocketAddr, ObjectId, usize),
    StoreError(SocketAddr, PlasmaError),
    ObjectsNotFound(SocketAddr, Vec<ObjectId>),
    ConnectionError(Option<SocketAddr>, std::io::Error),
}

impl ObjectSendError {
    pub fn response_code(&self) -> Option<u8> {
        match self {
            Self::ObjectDeletionScheduled(_, _) => Some(status_codes::OB_DELETION_SCHEDULED_ERR),
            Self::ObjectMetaTooLarge(_, _, _) => Some(status_codes::OB_META_TOO_LARGE_ERR),
            Self::ObjectDataTooLarge(_, _, _) => Some(status_codes::OB_DATA_TOO_LARGE_ERR),
            Self::ObjectsNotFound(_, _) => Some(status_codes::OB_NOT_FOUND_ERR),
            Self::StoreError(_, _) => Some(status_codes::PLASMA_STORE_ERR),
            Self::ConnectionError(_, _) => None,
        }
    }
}

impl Display for ObjectSendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectDeletionScheduled(peer, objects) => {
                write!(
                    f,
                    "failed to send objects to {}; object scheduled for deletion:",
                    peer
                )?;
                for oid in objects {
                    write!(f, "\n0x{}", hex::encode(oid))?
                }
            }
            Self::ObjectMetaTooLarge(peer, oid, _) => {
                write!(
                    f,
                    "failed to send objects to {}; metadata too larger for 0x{}",
                    peer,
                    hex::encode(oid),
                )?;
            }
            Self::ObjectDataTooLarge(peer, oid, _) => {
                write!(
                    f,
                    "failed to send objects to {}; data too larger for 0x{}",
                    peer,
                    hex::encode(oid),
                )?;
            }
            Self::ObjectsNotFound(peer, objects) => {
                write!(f, "failed to send objects to {}; objects not found:", peer)?;
                for oid in objects {
                    write!(f, "\n0x{}", hex::encode(oid))?
                }
            }
            Self::StoreError(peer, err) => {
                write!(
                    f,
                    "failed to send objects to {}; plasma store error: {}",
                    peer, err,
                )?;
            }
            Self::ConnectionError(peer, err) => match peer {
                Some(peer) => write!(f, "failed to send objects to {}: {}", peer, err)?,
                None => write!(f, "failed to send objects: {}", err)?,
            },
        };

        Ok(())
    }
}

impl std::error::Error for ObjectSendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ConnectionError(_, err) => Some(err.as_dyn_error()),
            Self::StoreError(_, err) => Some(err.as_dyn_error()),
            _ => None,
        }
    }
}

// OBJECT RECEIVE ERROR
// ================================================================================================

#[derive(Debug)]
pub enum ObjectReceiveError {
    AlreadyReceiving(SocketAddr, Vec<ObjectId>),
    AlreadyInStore(SocketAddr, Vec<ObjectId>),
    ObjectMetaTooLarge(SocketAddr, ObjectId, usize),
    ObjectDataTooLarge(SocketAddr, ObjectId, usize),
    ZeroLengthObjectData(SocketAddr, ObjectId),
    PeerError(SocketAddr, u8),
    StoreError(SocketAddr, PlasmaError),
    ConnectionError(Option<SocketAddr>, std::io::Error),
}

impl Display for ObjectReceiveError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyReceiving(peer, objects) => {
                write!(
                    f,
                    "did not request objects from {}; already receiving objects:",
                    peer
                )?;
                for oid in objects {
                    write!(f, "\n0x{}", hex::encode(oid))?
                }
            }
            Self::AlreadyInStore(peer, objects) => {
                write!(
                    f,
                    "did not request objects from {}; objects already in store:",
                    peer
                )?;
                for oid in objects {
                    write!(f, "\n0x{}", hex::encode(oid))?
                }
            }
            Self::ObjectMetaTooLarge(peer, oid, _) => {
                write!(
                    f,
                    "failed to receive objects from {}; metadata too larger for 0x{}",
                    peer,
                    hex::encode(oid),
                )?;
            }
            Self::ObjectDataTooLarge(peer, oid, _) => {
                write!(
                    f,
                    "failed to receiver objects from {}; data too larger for 0x{}",
                    peer,
                    hex::encode(oid),
                )?;
            }
            Self::ZeroLengthObjectData(peer, oid) => {
                write!(
                    f,
                    "failed to receiver objects from {}; zero-length data for 0x{}",
                    peer,
                    hex::encode(oid),
                )?;
            }
            Self::PeerError(peer, response_code) => {
                write!(f, "failed to receive objects from {}; ", peer)?;
                match *response_code {
                    status_codes::OB_DELETION_SCHEDULED_ERR => write!(f, "deletion in progress")?,
                    status_codes::OB_META_TOO_LARGE_ERR => write!(f, "object meta too large")?,
                    status_codes::OB_DATA_TOO_LARGE_ERR => write!(f, "object data too large")?,
                    status_codes::OB_NOT_FOUND_ERR => write!(f, "not found")?,
                    status_codes::PLASMA_STORE_ERR => write!(f, "peer plasma store error")?,
                    _ => write!(f, "unknown error code: {}", response_code)?,
                }
            }
            Self::StoreError(peer, err) => {
                write!(
                    f,
                    "failed to receive objects from {}; plasma store error: {}",
                    peer, err,
                )?;
            }
            Self::ConnectionError(peer, err) => match peer {
                Some(peer) => write!(f, "failed to receive objects from {}: {}", peer, err)?,
                None => write!(f, "failed to receive objects: {}", err)?,
            },
        };

        Ok(())
    }
}

impl std::error::Error for ObjectReceiveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ConnectionError(_, err) => Some(err.as_dyn_error()),
            Self::StoreError(_, err) => Some(err.as_dyn_error()),
            _ => None,
        }
    }
}

// REQUEST ERROR
// ================================================================================================

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("invalid request type {0}")]
    InvalidRequestType(u8),

    #[error("invalid subtask type {0}")]
    InvalidSubtaskType(u8),

    #[error("invalid peer address type {0}")]
    InvalidPeerAddressType(u8),

    #[error("object ID list is too long {0}")]
    ObjectIdListTooLong(usize),
}

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("peer connection failed: {0}")]
    PeerConnectionFailed(#[from] crate::Error),

    #[error("failed to send request to {peer}: {source}")]
    FailedToSendRequest {
        peer: SocketAddr,
        source: crate::Error,
    },
}
