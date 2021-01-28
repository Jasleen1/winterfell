use crate::{errors::RequestError, ObjectId, Result, OBJECT_ID_BYTES};
use std::{
    fmt::{Display, Formatter},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

// CONSTANTS
// ================================================================================================
const SYNC_TYPE_ID: u8 = 1;
const COPY_TYPE_ID: u8 = 2;
const TAKE_TYPE_ID: u8 = 3;

const IPV4_TYPE_ID: u8 = 4;
const IPV6_TYPE_ID: u8 = 6;

const MAX_OBJECT_ID_LIST_LEN: usize = 65_536; // 2^16

// REQUEST
// ================================================================================================

pub enum Request {
    Sync(Vec<SyncSubtask>),
    Copy(Vec<ObjectId>),
    Take(Vec<ObjectId>),
}

impl Request {
    /// Reads a request from the specified socket. This function will return when:
    /// * A well-formed request has been read.
    /// * The socket has been closed; in this case `None` will be returned.
    /// * The data read from the socket does not represent a valid request; in this case
    ///   an error will be returned
    pub async fn read_from(socket: &mut TcpStream) -> Result<Option<Self>> {
        // determine request type; also return `None` if the connection has been closed
        let request_type = match socket.read_u8().await {
            Ok(request_type) => request_type,
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // based on the type, read the rest of the request
        match request_type {
            SYNC_TYPE_ID => {
                let num_subtasks = socket.read_u16_le().await?;
                let mut subtasks = Vec::with_capacity(num_subtasks as usize);
                for _ in 0..num_subtasks {
                    let subtask = SyncSubtask::read_from(socket).await?;
                    subtasks.push(subtask);
                    // TODO: make sure there are no duplicate object IDs in subtasks
                }
                Ok(Some(Self::Sync(subtasks)))
            }
            COPY_TYPE_ID => {
                let object_ids = read_object_id_list(socket).await?;
                Ok(Some(Self::Copy(object_ids)))
            }
            TAKE_TYPE_ID => {
                let object_ids = read_object_id_list(socket).await?;
                Ok(Some(Self::Take(object_ids)))
            }
            _ => Err(RequestError::InvalidRequestType(request_type).into()),
        }
    }

    /// Writes this result into the socket.
    pub async fn write_into(&self, socket: &mut TcpStream) -> Result<()> {
        match self {
            Request::Sync(frames) => {
                socket.write_u8(SYNC_TYPE_ID).await?;
                socket.write_u16_le(frames.len() as u16).await?;
                for frame in frames.iter() {
                    frame.write_into(socket).await?;
                }
            }
            Request::Copy(object_ids) => {
                socket.write_u8(COPY_TYPE_ID).await?;
                write_object_id_list(object_ids, socket).await?;
            }
            Request::Take(object_ids) => {
                socket.write_u8(TAKE_TYPE_ID).await?;
                write_object_id_list(object_ids, socket).await?;
            }
        }
        Ok(())
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match self {
            Request::Sync(subtasks) => {
                write!(f, "SYNC")?;
                for subtask in subtasks.iter() {
                    write!(f, "\n{}", subtask)?;
                }
                write!(f, "")
            }
            Request::Copy(object_ids) => {
                write!(
                    f,
                    "COPY {:?}",
                    object_ids.iter().map(hex::encode).collect::<Vec<_>>()
                )
            }
            Request::Take(object_ids) => {
                write!(
                    f,
                    "TAKE {:?}",
                    object_ids.iter().map(hex::encode).collect::<Vec<_>>()
                )
            }
        }
    }
}

// SYNC SUBTASK
// ================================================================================================

pub enum SyncSubtask {
    Copy {
        from: SocketAddr,
        objects: Vec<ObjectId>,
    },
    Take {
        from: SocketAddr,
        objects: Vec<ObjectId>,
    },
}

impl SyncSubtask {
    /// Reads a SYNC request subtask from the specified socket.
    pub async fn read_from(socket: &mut TcpStream) -> Result<Self> {
        let subtask_type = socket.read_u8().await?;
        match subtask_type {
            COPY_TYPE_ID => {
                let from = read_socket_addr(socket).await?;
                let objects = read_object_id_list(socket).await?;
                Ok(SyncSubtask::Copy { from, objects })
            }
            TAKE_TYPE_ID => {
                let from = read_socket_addr(socket).await?;
                let objects = read_object_id_list(socket).await?;
                Ok(SyncSubtask::Take { from, objects })
            }
            _ => Err(RequestError::InvalidSubtaskType(subtask_type).into()),
        }
    }

    // Writes a SYNC request subtask into the specified socket.
    pub async fn write_into(&self, socket: &mut TcpStream) -> Result<()> {
        match self {
            SyncSubtask::Copy { from, objects } => {
                socket.write_u8(COPY_TYPE_ID).await?;
                write_peer_addr(from, socket).await?;
                write_object_id_list(objects, socket).await?;
            }
            SyncSubtask::Take { from, objects } => {
                socket.write_u8(TAKE_TYPE_ID).await?;
                write_peer_addr(from, socket).await?;
                write_object_id_list(objects, socket).await?;
            }
        }
        Ok(())
    }

    /// Gets a list of object IDs which will be received upon execution of this SYNC subtask.
    pub fn incoming_objects(&self) -> &[ObjectId] {
        match self {
            SyncSubtask::Copy { objects, .. } => &objects,
            SyncSubtask::Take { objects, .. } => &objects,
        }
    }
}

impl Display for SyncSubtask {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match self {
            SyncSubtask::Copy { from, objects } => {
                write!(
                    f,
                    "COPY {} {:?}",
                    from,
                    objects.iter().map(hex::encode).collect::<Vec<_>>()
                )
            }
            SyncSubtask::Take { from, objects } => {
                write!(
                    f,
                    "TAKE {} {:x?}",
                    from,
                    objects.iter().map(hex::encode).collect::<Vec<_>>()
                )
            }
        }
    }
}

// HELPER READERS
// ================================================================================================

/// Reads peer address from the specified socket.
async fn read_socket_addr(socket: &mut TcpStream) -> Result<SocketAddr> {
    let addr_type = socket.read_u8().await?;
    let port = socket.read_u16_le().await?;

    match addr_type {
        IPV4_TYPE_ID => {
            let addr = read_ipv4_address(socket).await?;
            Ok(SocketAddr::new(IpAddr::V4(addr), port))
        }
        IPV6_TYPE_ID => {
            let addr = read_ipv6_address(socket).await?;
            Ok(SocketAddr::new(IpAddr::V6(addr), port))
        }
        _ => Err(RequestError::InvalidPeerAddressType(addr_type).into()),
    }
}

/// Reads an IPv4 address from the specified socket.
async fn read_ipv4_address(socket: &mut TcpStream) -> Result<Ipv4Addr> {
    let a = socket.read_u32_le().await?;
    Ok(Ipv4Addr::new(
        a as u8,
        (a >> 8) as u8,
        (a >> 16) as u8,
        (a >> 24) as u8,
    ))
}

/// Reads an IPv6 address from the specified socket.
async fn read_ipv6_address(_socket: &mut TcpStream) -> Result<Ipv6Addr> {
    // TODO: add support for IPv6 addresses
    unimplemented!()
}

/// Reads a list of object IDs from the specified socket.
async fn read_object_id_list(socket: &mut TcpStream) -> Result<Vec<ObjectId>> {
    // determine number of object IDs
    let num_ids = socket.read_u16_le().await? as usize;

    // read all object ID bytes
    let mut result = vec![0u8; OBJECT_ID_BYTES * num_ids];
    socket.read_exact(&mut result).await?;

    // convert the vector of bytes into a vector of 20-byte arrays
    let mut v = std::mem::ManuallyDrop::new(result);
    let p = v.as_mut_ptr();
    let len = v.len() / OBJECT_ID_BYTES;
    let cap = v.capacity() / OBJECT_ID_BYTES;
    unsafe { Ok(Vec::from_raw_parts(p as *mut ObjectId, len, cap)) }
}

// HELPER WRITERS
// ================================================================================================

/// Writes a list of object IDs into the socket. Number of object IDs is written into the
/// socket first (as u16), followed by the actual object IDs.
async fn write_object_id_list(object_ids: &[ObjectId], socket: &mut TcpStream) -> Result<()> {
    if object_ids.len() > MAX_OBJECT_ID_LIST_LEN {
        return Err(RequestError::ObjectIdListTooLong(object_ids.len()).into());
    }
    socket.write_u16_le(object_ids.len() as u16).await?;
    for id in object_ids.iter() {
        socket.write_all(id).await?;
    }
    Ok(())
}

/// Writes socket address of the peer into the socket.
async fn write_peer_addr(peer_addr: &SocketAddr, socket: &mut TcpStream) -> Result<()> {
    match peer_addr {
        SocketAddr::V4(peer_addr) => {
            socket.write_u8(IPV4_TYPE_ID).await?;
            socket.write_u16(peer_addr.port()).await?;
            socket.write_all(&peer_addr.ip().octets()).await?;
        }
        SocketAddr::V6(peer_addr) => {
            socket.write_u8(IPV6_TYPE_ID).await?;
            socket.write_u16(peer_addr.port()).await?;
            socket.write_all(&peer_addr.ip().octets()).await?;
        }
    }
    Ok(())
}
