mod request;
pub use request::{Request, SyncSubtask};

mod store;
pub use store::Store;

// CONSTANTS
// ================================================================================================

pub const OBJECT_ID_BYTES: usize = 20;

// CONVENIENCE TYPES
// ================================================================================================

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

pub type ObjectId = [u8; OBJECT_ID_BYTES];
