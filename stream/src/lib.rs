mod request;
pub use request::{Request, SyncSubtask};

mod store;
pub use store::Store;

mod sender;
pub use sender::ObjectSender;

mod receiver;
pub use receiver::ObjectReceiver;

pub mod errors;
pub mod utils;

// CONSTANTS
// ================================================================================================

pub const OBJECT_ID_BYTES: usize = 20;

pub const MAX_META_SIZE: u64 = 65_536; // 2^16 or 64 KB
pub const MAX_DATA_SIZE: u64 = 281_474_976_710_656; // 2^44 or 256 TB

// CONVENIENCE TYPES
// ================================================================================================

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

pub type ObjectId = [u8; OBJECT_ID_BYTES];

// STATUS CODES
// ================================================================================================

pub mod status_codes {
    pub const BEGIN: u8 = 0x00;
    pub const SUCCESS: u8 = 0x41;
    pub const FAILURE: u8 = 0x46;
    pub const OB_DELETION_SCHEDULED_ERR: u8 = 0x50;
    pub const OB_META_TOO_LARGE_ERR: u8 = 0x51;
    pub const OB_DATA_TOO_LARGE_ERR: u8 = 0x52;
    pub const OB_NOT_FOUND_ERR: u8 = 0x53;
    pub const PLASMA_STORE_ERR: u8 = 0x54;
}
