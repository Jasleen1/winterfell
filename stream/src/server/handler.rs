use super::{status_codes, Request, Result, Store, SyncSubtask};
use std::sync::Arc;
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Semaphore};
use tracing::{debug, error};

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
    pub async fn run(&mut self) -> crate::Result<()> {
        // read requests until no more requests are available
        loop {
            // If no request was read then the peer closed the socket. There is no further work
            // to do and the task can be terminated.
            let request = match Request::read_from(&mut self.socket).await? {
                Some(request) => request,
                None => return Ok(()),
            };
            let peer_addr = self.socket.peer_addr()?;
            debug!("Received request from {}\n{}", peer_addr, request);

            // process the request
            match request {
                Request::Copy(object_ids) => {
                    // for COPY request, just send the objects to the requesting peer
                    self.store
                        .build_sender(peer_addr, object_ids, false)
                        .run(&mut self.socket)
                        .await?;
                }
                Request::Take(object_ids) => {
                    // for TAKE request, send the objects, but also delete them afterwards
                    self.store
                        .build_sender(peer_addr, object_ids, true)
                        .run(&mut self.socket)
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

                    // wait for all subtasks to finish, and write the result of each subtasks
                    // (success or error) into the socket
                    for handle in handles {
                        match handle.await {
                            Ok(result) => match result {
                                Ok(_) => self.socket.write_u8(status_codes::SUCCESS).await?,
                                Err(err) => self.handle_sync_subtask_error(err).await?,
                            },
                            Err(err) => self.handle_sync_subtask_error(err.into()).await?,
                        }
                    }
                }
            };
        }
    }

    async fn handle_sync_subtask_error(&mut self, err: crate::Error) -> Result<()> {
        error!("sync subtask failed: {}", err);
        self.socket.write_u8(status_codes::FAILURE).await?;
        Ok(())
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
        SyncSubtask::Copy { from, objects } | SyncSubtask::Take { from, objects } => {
            // build the receiver and prepare it to receive objects
            let receiver = store.build_receiver(from, objects.clone());
            receiver.prepare()?;

            // open the socket and send the request
            let mut socket = TcpStream::connect(from).await?;
            let request = Request::Copy(objects);
            request.write_into(&mut socket).await?;

            // read the response and close connection when done
            receiver.run(&mut socket).await?;
            socket.shutdown().await?;
        }
    }
    Ok(())
}
