use crate::{ObjectId, Result, OBJECT_ID_BYTES};
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

// REQUEST
// ================================================================================================

pub enum Request {
    Sync(Vec<SyncSubtask>),
    Copy(Vec<ObjectId>),
    Take(Vec<ObjectId>),
}

impl Request {
    pub async fn read_from(stream: &mut TcpStream) -> Result<Option<Self>> {
        let request_type = match stream.read_u8().await {
            Ok(request_type) => request_type,
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        match request_type {
            SYNC_TYPE_ID => {
                let num_subtasks = stream.read_u16_le().await?;
                let mut subtasks = Vec::with_capacity(num_subtasks as usize);
                for _ in 0..num_subtasks {
                    let subtask = SyncSubtask::read_from(stream).await?;
                    subtasks.push(subtask);
                }
                Ok(Some(Self::Sync(subtasks)))
            }
            COPY_TYPE_ID => {
                let object_ids = parse_object_id_list(stream).await?;
                Ok(Some(Self::Copy(object_ids)))
            }
            TAKE_TYPE_ID => {
                let object_ids = parse_object_id_list(stream).await?;
                Ok(Some(Self::Take(object_ids)))
            }
            _ => unimplemented!(),
        }
    }

    pub async fn write_into(&self, stream: &mut TcpStream) -> Result<()> {
        match self {
            Request::Sync(frames) => {
                stream.write_u8(SYNC_TYPE_ID).await?;
                stream.write_u16_le(frames.len() as u16).await?;
                for frame in frames.iter() {
                    frame.write_into(stream).await?;
                }
            }
            Request::Copy(object_ids) => {
                stream.write_u8(COPY_TYPE_ID).await?;
                write_object_id_list(object_ids, stream).await?;
            }
            Request::Take(object_ids) => {
                stream.write_u8(TAKE_TYPE_ID).await?;
                write_object_id_list(object_ids, stream).await?;
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
                write!(f, "TAKE {:x?}", object_ids)
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
    pub async fn read_from(stream: &mut TcpStream) -> Result<Self> {
        let subtask_type = stream.read_u8().await?;
        match subtask_type {
            COPY_TYPE_ID => {
                let from = read_socket_addr(stream).await?;
                let objects = parse_object_id_list(stream).await?;
                Ok(SyncSubtask::Copy { from, objects })
            }
            TAKE_TYPE_ID => {
                let from = read_socket_addr(stream).await?;
                let objects = parse_object_id_list(stream).await?;
                Ok(SyncSubtask::Take { from, objects })
            }
            _ => unimplemented!(),
        }
    }

    pub async fn write_into(&self, stream: &mut TcpStream) -> Result<()> {
        match self {
            SyncSubtask::Copy { from, objects } => {
                stream.write_u8(COPY_TYPE_ID).await?;
                write_socket_addr(from, stream).await?;
                write_object_id_list(objects, stream).await?;
            }
            SyncSubtask::Take { from, objects } => {
                stream.write_u8(TAKE_TYPE_ID).await?;
                write_socket_addr(from, stream).await?;
                write_object_id_list(objects, stream).await?;
            }
        }
        Ok(())
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

// HELPER PARSERS
// ================================================================================================

async fn read_socket_addr(stream: &mut TcpStream) -> Result<SocketAddr> {
    let addr_type = stream.read_u8().await?;
    let port = stream.read_u16_le().await?;

    match addr_type {
        IPV4_TYPE_ID => {
            let addr = parse_ipv4(stream).await?;
            Ok(SocketAddr::new(IpAddr::V4(addr), port))
        }
        IPV6_TYPE_ID => {
            let addr = parse_ipv6(stream).await?;
            Ok(SocketAddr::new(IpAddr::V6(addr), port))
        }
        _ => unimplemented!(),
    }
}

async fn parse_ipv4(stream: &mut TcpStream) -> Result<Ipv4Addr> {
    let a = stream.read_u32_le().await?;
    Ok(Ipv4Addr::new(
        a as u8,
        (a >> 8) as u8,
        (a >> 16) as u8,
        (a >> 24) as u8,
    ))
}

async fn parse_ipv6(_stream: &mut TcpStream) -> Result<Ipv6Addr> {
    // TODO
    unimplemented!()
}

async fn parse_object_id_list(stream: &mut TcpStream) -> Result<Vec<[u8; 20]>> {
    // determine number of object IDs
    let num_ids = stream.read_u16_le().await? as usize;

    // read all object ID bytes
    let mut result = vec![0u8; OBJECT_ID_BYTES * num_ids];
    stream.read_exact(&mut result).await?;

    // convert the vector of bytes into a vector of 20-byte arrays
    let mut v = std::mem::ManuallyDrop::new(result);
    let p = v.as_mut_ptr();
    let len = v.len() / OBJECT_ID_BYTES;
    let cap = v.capacity() / OBJECT_ID_BYTES;
    unsafe {
        Ok(Vec::from_raw_parts(
            p as *mut [u8; OBJECT_ID_BYTES],
            len,
            cap,
        ))
    }
}

// HELPER READERS
// ================================================================================================
async fn write_object_id_list(object_ids: &[ObjectId], stream: &mut TcpStream) -> Result<()> {
    stream.write_u16_le(object_ids.len() as u16).await?;
    for id in object_ids.iter() {
        stream.write_all(id).await?;
    }
    Ok(())
}

async fn write_socket_addr(socket: &SocketAddr, stream: &mut TcpStream) -> Result<()> {
    match socket {
        SocketAddr::V4(socket) => {
            stream.write_u8(IPV4_TYPE_ID).await?;
            stream.write_u16(socket.port()).await?;
            stream.write_all(&socket.ip().octets()).await?;
        }
        SocketAddr::V6(socket) => {
            stream.write_u8(IPV6_TYPE_ID).await?;
            stream.write_u16(socket.port()).await?;
            stream.write_all(&socket.ip().octets()).await?;
        }
    }
    Ok(())
}
