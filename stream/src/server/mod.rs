use stream::{Request, Result, Store, SyncSubtask};
use structopt::StructOpt;
use tokio::signal;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

mod listener;
use listener::Listener;

mod handler;
use handler::Handler;

// CONSTANTS
// ================================================================================================

const DEFAULT_PORT: &str = "2021";
const DEFAULT_PLASMA_SOCKET: &str = "/tmp/plasma";
const DEFAULT_PLASMA_TIMEOUT: &str = "10";
const DEFAULT_MAX_CONNECTIONS: &str = "128";

const PLASMA_CONNECT_RETRIES: u32 = 4;

// COMMAND LINE ARGUMENTS
// ================================================================================================

#[derive(StructOpt, Debug)]
#[structopt(name = "porter", version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"), about = "Plasma object porter")]
pub struct ServerOptions {
    /// TCP port for the porter to listen on
    #[structopt(short, long, default_value=DEFAULT_PORT)]
    port: String,

    /// Maximum number of TCP connections accepted by this server
    #[structopt(short="c", long, default_value=DEFAULT_MAX_CONNECTIONS)]
    max_connections: u32,

    /// Unix socket bound to the local Plasma Store
    #[structopt(short="s", long, default_value=DEFAULT_PLASMA_SOCKET)]
    plasma_socket: String,

    /// The amount of time in milliseconds to wait before requests to Plasma Store time out.
    #[structopt(short="t", long, default_value=DEFAULT_PLASMA_TIMEOUT)]
    plasma_timeout: i64,
}

// PROGRAM ENTRY POINT
// ================================================================================================

#[tokio::main]
pub async fn main() -> Result<()> {
    // turn tracing on
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // listen to shutdown signal
    let shutdown = signal::ctrl_c();

    // read command-line args
    let options = ServerOptions::from_args();

    // create the listener
    let mut server = Listener::new(options).await?;

    // TODO: add comment
    tokio::select! {
        res = server.start() => {
            // If an error is received here, accepting connections from the TCP listener failed
            // multiple times and the server is giving up and shutting down.
            //
            // Errors encountered when handling individual connections do not bubble up to
            // this point.
            if let Err(err) = res {
                error!(cause = %err, "failed to accept");
            }
        }
        _ = shutdown => {
            // The shutdown signal has been received.
            info!("shutting down");
        }
    }

    Ok(())
}
