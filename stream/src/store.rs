use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::debug;

use crate::{ObjectId, Result};

#[derive(Debug)]
pub struct Store {}

impl Store {
    pub fn new() -> Self {
        Store {}
    }

    pub async fn send_objects(
        &self,
        object_ids: &[ObjectId],
        stream: &mut TcpStream,
        delete_objects: bool,
    ) -> Result<()> {
        debug!(
            "sending {} objects to {}",
            object_ids.len(),
            stream.peer_addr()?
        );

        // make sure all objects exist
        for _id in object_ids {
            // lock the object
            // get object from plasma store
            // write object into the connection
            let ob = get_test_object();
            let head = ob.meta().len() | (ob.data.len() << 16);
            stream.write_u64_le(head as u64).await?;
            stream.write_all(ob.meta()).await?;
            stream.write_all(ob.data()).await?;

            if delete_objects {
                // delete the object
            }
        }

        Ok(())
    }

    pub async fn receive_objects(
        &self,
        object_ids: &[ObjectId],
        stream: &mut TcpStream,
    ) -> Result<()> {
        debug!(
            "receiving {} objects from {}",
            object_ids.len(),
            stream.peer_addr()?
        );

        for _id in object_ids {
            let head = stream.read_u64_le().await?;
            let meta_size = (head as u16) as usize;
            let data_size = (head >> 16) as usize;

            let mut meta_buf = vec![0u8; meta_size];
            stream.read_exact(&mut meta_buf).await?;

            // create object
            // stream data to object
            let mut data_buf = vec![0u8; data_size];
            stream.read_exact(&mut data_buf).await?;

            let ob = ObjectBuffer {
                meta: meta_buf,
                data: data_buf,
            };
            debug!("received {:?}", ob);

            // seal the object
        }

        Ok(())
    }
}

// TEMP FUNCTIONS
// ================================================================================================

#[derive(Debug)]
struct ObjectBuffer {
    meta: Vec<u8>,
    data: Vec<u8>,
}

impl ObjectBuffer {
    pub fn id(&self) -> ObjectId {
        [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ]
    }

    pub fn meta(&self) -> &[u8] {
        &self.meta
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

fn get_test_object() -> ObjectBuffer {
    ObjectBuffer {
        meta: vec![1, 2, 3, 4],
        data: vec![1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31],
    }
}
