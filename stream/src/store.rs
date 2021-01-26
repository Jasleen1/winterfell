use std::{
    collections::HashSet,
    convert::TryInto,
    sync::{Arc, Mutex},
};

use crate::{
    errors::{ObjectReceiveError, ObjectSendError},
    ObjectId, Result,
};
use plasma::{ObjectBuffer, PlasmaClient};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::{debug, info};

// CONSTANTS
// ================================================================================================

const MAX_META_SIZE: u64 = 65_536; // 2^16 or 64 KB
const MAX_DATA_SIZE: u64 = 281_474_976_710_656; // 2^44 or 256 TB

// OBJECT STORE WRAPPER
// ================================================================================================

#[derive(Debug, Clone)]
pub struct Store {
    /// Connection to the Plasma Store. We put it into an Arc because it can be accessed from
    /// multiple threads concurrently, and we don't want to clone the connection for each thread.
    plasma_client: Arc<PlasmaClient>,

    /// Maximum time allocated to retrieving objects from the store.
    timeout_ms: i64,

    /// A set of IDs for objects which are in the process of being received. This is used to
    /// make sure two separate requests don't try to receive the same object.
    receiving: Arc<Mutex<HashSet<ObjectId>>>,

    /// A set of IDs for objects which are scheduled to be deleted. This is used to make sure
    /// two separate requests don't try to delete the same object from the store.
    deleting: Arc<Mutex<HashSet<ObjectId>>>,
}

impl Store {
    pub fn new(plasma_client: PlasmaClient, timeout_ms: i64) -> Self {
        Store {
            plasma_client: Arc::new(plasma_client),
            timeout_ms,
            receiving: Arc::new(Mutex::new(HashSet::new())),
            deleting: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    // SENDING OBJECTS
    // --------------------------------------------------------------------------------------------

    /// Reads the specified objects from the local Plasma Store and sends them into the
    /// specified socket. If `delete_after_send` = true, the objects are deleted from local
    /// Plasma Store.
    pub async fn send_objects(
        &self,
        object_ids: &[ObjectId],
        socket: &mut TcpStream,
        delete_after_send: bool,
    ) -> Result<()> {
        let peer_address = socket.peer_addr()?;
        info!("sending {} objects to {}", object_ids.len(), peer_address);

        // make sure none of the objects to be sent are currently scheduled for deletion;
        // if delete_after_send = true and none of the objects is scheduled for deletion,
        // this will also add the object IDs to the set of objects scheduled for deletion
        let in_deleting = self.check_deleting(object_ids, delete_after_send);
        if !in_deleting.is_empty() {
            return Err(Box::new(ObjectSendError::DeletionScheduled(in_deleting)));
        }

        // get all objects from the Plasma Store; this also ensures that all requested
        // objects exist locally.
        let objects = match self.get_objects(object_ids) {
            Ok(objects) => objects,
            Err(e) => {
                // if there was some error, make sure we clear the deleting set
                if delete_after_send {
                    self.remove_from_deleting(object_ids);
                }
                // error returned from get_objects() is already an ObjectSendError
                return Err(e);
            }
        };

        let mut bytes_sent = 0;
        for ob in objects.iter() {
            match send_object(ob, socket).await {
                Ok(()) => {
                    debug!("sent object {} to {}", ob, peer_address);
                    bytes_sent += ob.size();
                }
                Err(e) => {
                    // if there was some error, make sure we clear the deleting set
                    if delete_after_send {
                        self.remove_from_deleting(object_ids);
                    }
                    return Err(Box::new(ObjectSendError::Unknown(e)));
                }
            }
        }

        info!(
            "sent {} objects ({} bytes) to {}",
            object_ids.len(),
            bytes_sent,
            peer_address
        );

        // if asked, delete the objects from the local plasma store
        if delete_after_send {
            let plasma_object_ids = map_object_ids(object_ids);
            match self.plasma_client.delete_many(&plasma_object_ids) {
                Ok(()) => {
                    self.remove_from_deleting(object_ids);
                }
                Err(e) => {
                    self.remove_from_deleting(object_ids);
                    return Err(Box::new(ObjectSendError::Unknown(e.into())));
                }
            }
        }

        Ok(())
    }

    fn get_objects(&self, object_ids: &[ObjectId]) -> Result<Vec<ObjectBuffer>> {
        let plasma_object_ids = map_object_ids(object_ids);
        match self
            .plasma_client
            .get_many(&plasma_object_ids, self.timeout_ms)
        {
            Ok(objects) => {
                // make sure all objects were retrieved
                let mut missing = Vec::new();
                let mut result = Vec::with_capacity(objects.len());
                for ob in objects.into_iter() {
                    match ob {
                        Some(ob) => result.push(ob),
                        None => missing.push(ob),
                    }
                }

                if !missing.is_empty() {
                    // TODO: throw error
                }

                Ok(result)
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Returns a vector of any `object_ids` found in the set of objects scheduled for deletion.
    fn check_deleting(&self, object_ids: &[ObjectId], will_delete: bool) -> Vec<ObjectId> {
        // ensure thread-safety by acquiring a lock to the set of objects scheduled for deletion;
        // `unwrap()` is OK here because no thread will panic wile holding the lock.
        let mut deleting = self.deleting.lock().unwrap();

        let mut result = Vec::new();
        for oid in object_ids {
            if deleting.contains(oid) {
                result.push(*oid);
            }
        }

        if result.is_empty() && will_delete {
            deleting.extend(object_ids.iter());
        }

        result
    }

    fn remove_from_deleting(&self, object_ids: &[ObjectId]) {
        let mut deleting = self.deleting.lock().unwrap();
        for oid in object_ids.iter() {
            deleting.remove(oid);
        }
    }

    // RECEIVING OBJECTS
    // --------------------------------------------------------------------------------------------

    /// Reads objects from the specified socket and saves them into the local Plasma Store;
    /// the objects are assumed to be order in the order specified by `object_ids` list.
    pub async fn receive_objects(
        &self,
        object_ids: &[ObjectId],
        socket: &mut TcpStream,
    ) -> Result<()> {
        // save peer address here for debugging purposes
        let peer_address = socket.peer_addr()?;
        info!(
            "receiving {} objects from {}",
            object_ids.len(),
            peer_address
        );

        // mark the objects as being received; if any of the object IDs is already marked
        // as being received, this will return an error; this is to make sure we don't try
        // to receive the same object twice (e.g. from two different peers)
        self.add_to_receiving(object_ids)?;

        // make sure the objects are not already in the store
        let plasma_object_ids = map_object_ids(object_ids);
        let in_store = self.plasma_client.contains_many(&plasma_object_ids)?;
        if !in_store.is_empty() {
            self.remove_from_receiving(object_ids);
            let in_store = in_store
                .into_iter()
                .map(|oid| oid.to_bytes().try_into().unwrap())
                .collect();
            return Err(Box::new(ObjectReceiveError::AlreadyInStore(in_store)));
        }

        let mut bytes_received = 0;

        for oid in plasma_object_ids.iter() {
            match receive_object(&self.plasma_client, oid, socket).await {
                Ok(ob) => {
                    debug!("received object {} from {}", ob, peer_address);
                    bytes_received += ob.size();
                }
                Err(e) => {
                    // TODO: delete already received objects?
                    // objects will not be received - remove them from the receiving set
                    self.remove_from_receiving(object_ids);
                    return Err(Box::new(ObjectReceiveError::Unknown(e)));
                }
            };
        }

        // all objects have been received - so, remove them from the receiving set
        self.remove_from_receiving(object_ids);
        info!(
            "received {} objects ({} bytes) from {}",
            object_ids.len(),
            bytes_received,
            peer_address
        );
        Ok(())
    }

    /// Adds all IDs from `object_ids` into the set of objects which are currently being received;
    /// if any of the IDs is already in the list, this will return an error.
    fn add_to_receiving(&self, object_ids: &[ObjectId]) -> Result<()> {
        // ensure thread-safety by acquiring a lock to the set of objects being received;
        // `unwrap()` is OK here because no thread will panic wile holding the lock.
        let mut receiving = self.receiving.lock().unwrap();

        // if any of the object IDs is already in the store, return an error
        let mut duplicates = Vec::new();
        for oid in object_ids.iter() {
            if receiving.contains(oid) {
                duplicates.push(*oid);
            }
        }

        if !duplicates.is_empty() {
            return Err(Box::new(ObjectReceiveError::AlreadyReceiving(duplicates)));
        }

        // add all object IDs to the set and return
        receiving.extend(object_ids.iter());
        Ok(())
    }

    fn remove_from_receiving(&self, object_ids: &[ObjectId]) {
        let mut receiving = self.receiving.lock().unwrap();
        for oid in object_ids.iter() {
            receiving.remove(oid);
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Creates a u64 object header. The object header consists of a 16-bit value describing the size
/// of the metadata, and a 48-bit value describing the size of that data. Thus, object metadata is
/// limited to at most 64 KB, while object data can be potentially as larger as 256 TB.
fn build_object_header(ob: &ObjectBuffer) -> Result<u64> {
    let meta_size = ob.meta().len() as u64;
    if meta_size > MAX_META_SIZE {
        return Err(Box::new(ObjectReceiveError::ObjectMetaTooLarge(
            meta_size as usize,
        )));
    }
    let data_size = ob.data().len() as u64;
    if data_size > MAX_DATA_SIZE {
        return Err(Box::new(ObjectReceiveError::ObjectDataTooLarge(
            meta_size as usize,
        )));
    }
    Ok(meta_size | (data_size << 16))
}

/// Breaks object header into metadata size (lower 16 bits) and data size (upper 48 bits).
fn parse_object_header(header: u64) -> (usize, usize) {
    let meta_size = (header as u16) as usize;
    let data_size = (header >> 16) as usize;
    (meta_size, data_size)
}

/// Converts a list of 20-byte arrays into plasma store object IDs.
fn map_object_ids(object_ids: &[ObjectId]) -> Vec<plasma::ObjectId> {
    object_ids
        .iter()
        .map(|oid| plasma::ObjectId::new(*oid))
        .collect()
}

/// Reads a single object from the socket and saves it under the specified 'oid'
/// into the local Plasma Store.
async fn receive_object<'a>(
    pc: &'a PlasmaClient,
    oid: &plasma::ObjectId,
    socket: &mut TcpStream,
) -> Result<ObjectBuffer<'a>> {
    // read the header to determine size of object data and metadata
    let header = socket.read_u64_le().await?;
    let (meta_size, data_size) = parse_object_header(header);

    // read the metadata from the socket and save it into a vector
    let mut meta_buf = vec![0u8; meta_size];
    socket.read_exact(&mut meta_buf).await?;

    // create object in the plasma store
    let mut ob = pc.create(oid.clone(), data_size, &meta_buf)?;

    // read object data from the socket and save it into the object buffer
    let data_buf = ob.data_mut();
    socket.read_exact(data_buf).await?;

    // seal the object to make it available to other clients
    ob.seal()?;

    Ok(ob)
}

/// Writes the object into the socket; the object is written as follows:
/// * first object header (data and meta size) is written as u64
/// * then, object metadata is written,
/// * and finally, object data buffer is written
async fn send_object(ob: &ObjectBuffer<'_>, socket: &mut TcpStream) -> Result<()> {
    // write object header into the socket; the header contains sizes of
    // data and metadata encoded into a single u64
    let header = build_object_header(&ob)?;
    socket.write_u64_le(header).await?;

    // write both data and metadata into the socket
    socket.write_all(ob.meta()).await?;
    socket.write_all(ob.data()).await?;

    Ok(())
}

// TEMP FUNCTIONS
// ================================================================================================

#[test]
fn plasma_insert_objects() {
    let oid = plasma::ObjectId::new([
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    ]);

    let pc1 = PlasmaClient::new("/tmp/plasma", 0).unwrap();
    let pc2 = PlasmaClient::new("/tmp/plasma2", 0).unwrap();

    if !pc1.contains(&oid).unwrap() {
        let meta = vec![1, 2, 3, 4];
        let data_size = 1024 * 1024 * 1024;
        let mut ob = pc1.create(oid.clone(), data_size, &meta).unwrap();

        let data_buf = ob.data_mut();
        for i in 0..data_size {
            data_buf[i] = i as u8;
        }
        ob.seal().unwrap();
        println!("object created in p1");

        if pc2.contains(&oid).unwrap() {
            println!("object exists in p2");
        } else {
            println!("object does not exist in p2");
        }
    } else {
        let ob = pc1.get(oid.clone(), 5).unwrap().unwrap();
        println!(
            "object exists in p1, size: {} MB",
            ob.data().len() / 1024 / 1024
        );
        if pc2.contains(&oid).unwrap() {
            pc2.delete(&oid).unwrap();
            println!("deleted object from p2");
        } else {
            println!("object does not exist in p2");
        }
    }

    assert!(false);
}
