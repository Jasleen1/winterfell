use std::sync::Arc;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    time::{self, Duration},
};
use tracing::{debug, error, info};

use super::{Cla, Handler, Result, Store};

#[derive(Debug)]
pub struct Listener {
    listener: TcpListener,
    store: Arc<Store>,
    limit_connections: Arc<Semaphore>,
}

impl Listener {
    pub async fn new(options: Cla) -> Result<Listener> {
        // Bind a TCP listener
        let address = format!("127.0.0.1:{}", options.port);
        info!("starting server on {}", address);
        let listener = TcpListener::bind(&address).await?;

        // create a semaphore to enforce connection limit
        let limit_connections = Arc::new(Semaphore::new(options.max_connections as usize));

        // create an object store
        let store = Arc::new(Store::new());

        Ok(Listener {
            listener,
            limit_connections,
            store,
        })
    }

    /// Start listening for inbound connections. For each inbound connection, spawn a
    /// task to process that connection.
    ///
    /// Returns `Err` if accepting returns an error.
    pub async fn start(&mut self) -> Result<()> {
        info!("accepting inbound connections");

        loop {
            // Wait for a permit to become available
            //
            // `acquire()` returns `Err` when the semaphore has been closed. We don't ever
            // close the sempahore, so `unwrap()` is safe.
            self.limit_connections.acquire().await.unwrap().forget();

            // Accept a new socket. This will attempt to perform error handling. The `accept`
            // method internally attempts to recover errors, so an error here is non-recoverable.
            let socket = self.accept().await?;
            debug!("accepted connection from {}", socket.peer_addr().unwrap());

            // Create the necessary per-connection handler state.
            let mut handler = Handler {
                store: self.store.clone(),
                socket,
                // The connection state needs a handle to the max connections
                // semaphore. When the handler is done processing the
                // connection, a permit is added back to the semaphore.
                limit_connections: self.limit_connections.clone(),
            };

            // Spawn a new task to process the connections. Tokio tasks are like
            // asynchronous green threads and are executed concurrently.
            tokio::spawn(async move {
                // Process the connection. If an error is encountered, log it.
                if let Err(err) = handler.run().await {
                    error!(cause = ?err, "connection error");
                }
            });
        }
    }

    /// Accept an inbound connection.
    ///
    /// Errors are handled by backing off and retrying. An incremental backoff strategy is used.
    /// After the first failure, the task waits for 1 second. After the second failure, the task
    /// waits for 2 seconds. Each subsequent failure increases the wait time by 1 second. If
    /// accepting fails on the 5th try after waiting for 4 seconds, an error is returned.
    async fn accept(&mut self) -> crate::Result<TcpStream> {
        let mut backoff = 1;

        loop {
            // Perform the accept operation. If a socket is successfully accepted, return it.
            // Otherwise, save the error.
            match self.listener.accept().await {
                Ok((socket, _)) => return Ok(socket),
                Err(err) => {
                    // If accept has failed too many times. Return the error.
                    if backoff > 4 {
                        return Err(err.into());
                    }
                }
            }

            // Pause execution until the back off period elapses.
            time::sleep(Duration::from_secs(backoff)).await;

            // Increment the back off
            backoff += 1;
        }
    }
}
