use super::{Request, Result, Store, SyncSubtask};
use std::sync::Arc;
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Semaphore};
use tracing::{debug, instrument};

// CONNECTION HANDLER
// ================================================================================================

/// Per-connection handler
#[derive(Debug)]
pub struct Handler {
    socket: TcpStream,
    store: Arc<Store>,
    limit_connections: Arc<Semaphore>,
}

impl Handler {
    pub fn new(socket: TcpStream, store: Arc<Store>, limit_connections: Arc<Semaphore>) -> Self {
        Handler {
            socket,
            store,
            limit_connections,
        }
    }

    /// Process a single connection.
    ///
    /// Requests are read from the socket and processed until there are no requests left.
    #[instrument(skip(self))]
    pub async fn run(&mut self) -> crate::Result<()> {
        // read requests until no more requests are available
        loop {
            let maybe_request = Request::read_from(&mut self.socket).await?;

            // If no request was read then the peer closed the socket. There is no
            // further work to do and the task can be terminated.
            let request = match maybe_request {
                Some(request) => request,
                None => return Ok(()),
            };
            debug!(
                "Received request from {}\n{}",
                self.socket.peer_addr()?,
                request
            );

            // process the request
            match request {
                Request::Copy(object_ids) => {
                    // for COPY request, just send the objects to the requesting peer
                    // TODO: handle errors
                    self.store
                        .send_objects(&object_ids, &mut self.socket, false)
                        .await?;
                }
                Request::Take(object_ids) => {
                    // for TAKE request, send the objects, but also delete them afterwards
                    // TODO: handle errors
                    self.store
                        .send_objects(&object_ids, &mut self.socket, true)
                        .await?;
                }
                Request::Sync(subtasks) => {
                    // for SYNC request, use separate task to fullfil each sync subtask; this
                    // is done to enable parallel streaming of objects from multiple peers
                    let mut handles = Vec::new();
                    for subtask in subtasks.into_iter() {
                        let store = self.store.clone();
                        let handle =
                            tokio::spawn(async move { handle_sync_subtask(store, subtask).await });
                        handles.push(handle);
                    }

                    // wait for all subtasks to finish
                    for handle in handles {
                        // TODO: handle errors
                        handle.await??;
                    }

                    // TODO: respond back to the original request
                    self.socket.write_u8(65).await?;
                }
            };
        }
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        // Add a permit back to the semaphore. Doing so unblocks the listener if the max
        // number of connections has been reached.
        self.limit_connections.add_permits(1);
        debug!("closed connection to {}", self.socket.peer_addr().unwrap());
    }
}

// HELPER FUNCTIONS
// ================================================================================================
async fn handle_sync_subtask(store: Arc<Store>, subtask: SyncSubtask) -> Result<()> {
    match subtask {
        SyncSubtask::Copy { from, objects } => {
            let mut socket = TcpStream::connect(from).await?;
            let request = Request::Copy(objects.clone()); // TODO: get rid of clone
            request.write_into(&mut socket).await?;
            store.receive_objects(&objects, &mut socket).await?;
            socket.shutdown().await?;
        }
        SyncSubtask::Take { from, objects } => {
            let mut socket = TcpStream::connect(from).await?;
            let request = Request::Take(objects.clone()); // TODO: get rid of clone
            request.write_into(&mut socket).await?;
            store.receive_objects(&objects, &mut socket).await?;
            socket.shutdown().await?;
        }
    }
    Ok(())
}
