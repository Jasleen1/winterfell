use super::{ObjectId, ObjectReceiver, ObjectSender};
use plasma::PlasmaClient;
use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

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

    /// Returns a new ObjectSender for sending objects with the specified IDs.
    pub fn build_sender(
        &self,
        peer_addr: SocketAddr,
        object_ids: Vec<ObjectId>,
        delete_after_send: bool,
    ) -> ObjectSender {
        ObjectSender {
            peer_addr,
            object_ids,
            delete_after_send,
            plasma_client: self.plasma_client.clone(),
            timeout_ms: self.timeout_ms,
            deleting: self.deleting.clone(),
        }
    }

    /// Returns a new ObjectReceiver for receiving objects with the specified IDs.
    pub fn build_receiver(
        &self,
        peer_addr: SocketAddr,
        object_ids: Vec<ObjectId>,
    ) -> ObjectReceiver {
        ObjectReceiver {
            peer_addr,
            object_ids,
            plasma_client: self.plasma_client.clone(),
            receiving: self.receiving.clone(),
        }
    }
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
