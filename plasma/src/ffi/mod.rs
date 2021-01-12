#[cfg(test)]
mod tests;

#[cxx::bridge(namespace = plasma)]
pub mod ffi {

    /// Object buffer data structure.
    struct ObjectBuffer {
        /// The data buffer.
        data: SharedPtr<Buffer>,
        /// The metadata buffer.
        metadata: SharedPtr<Buffer>,
        /// The device number.
        device_num: i32,
    }

    #[derive(Debug)]
    pub struct ArrowStatus {
        code: StatusCode,
        msg: String,
    }

    #[derive(Debug)]
    enum StatusCode {
        OK,
        OutOfMemory,
        KeyError,
        TypeError,
        Invalid,
        IOError,
        CapacityError,
        IndexError,
        UnknownError,
        NotImplemented,
        SerializationError,
        RError,                    // 13
        CodeGenError,              // 40
        ExpressionValidationError, // 41
        ExecutionError,            // 42
        AlreadyExists,             // 45
    }

    unsafe extern "C++" {
        include!("src/ffi/ffi.h");

        type ObjectID;

        fn oid_from_binary(binary: &[u8]) -> UniquePtr<ObjectID>;
        fn oid_to_binary(oid: &ObjectID) -> &[u8];
        fn oid_to_hex(oid: &ObjectID) -> String;
        fn oid_size(oid: &ObjectID) -> i64;
        fn oid_equals(oid1: &ObjectID, oid2: &ObjectID) -> bool;

        #[namespace = "arrow"]
        type Buffer;

        fn get_buffer_data<'a>(buffer: SharedPtr<Buffer>) -> &'a [u8];
        fn get_buffer_data_mut<'a>(buffer: SharedPtr<Buffer>) -> &'a mut [u8];
        fn is_buffer_mutable(buffer: SharedPtr<Buffer>) -> bool;

        #[namespace = "arrow"]
        type MutableBuffer;

        type ObjectBuffer;
        fn new_obj_buffer() -> UniquePtr<ObjectBuffer>;

        type PlasmaClient;

        fn new_plasma_client() -> UniquePtr<PlasmaClient>;
        fn connect(
            pc: Pin<&mut PlasmaClient>,
            store_socket_name: &str,
            num_retries: u32,
        ) -> ArrowStatus;

        fn set_client_options(
            pc: Pin<&mut PlasmaClient>,
            client_name: &str,
            output_memory_quote: i64,
        ) -> ArrowStatus;

        fn create(
            pc: Pin<&mut PlasmaClient>,
            ob: Pin<&mut ObjectBuffer>,
            oid: &ObjectID,
            data_size: i64,
            metadata: &[u8],
        ) -> ArrowStatus;

        fn create_and_seal(
            pc: Pin<&mut PlasmaClient>,
            oid: &ObjectID,
            data: &[u8],
            metadata: &[u8],
        ) -> ArrowStatus;

        fn get(
            pc: Pin<&mut PlasmaClient>,
            oid: &ObjectID,
            timeout_ms: i64,
            ob: Pin<&mut ObjectBuffer>,
        ) -> ArrowStatus;

        fn multi_get(
            pc: Pin<&mut PlasmaClient>,
            oids: &CxxVector<ObjectID>,
            timeout_ms: i64,
            obs: Pin<&mut CxxVector<ObjectBuffer>>,
        ) -> ArrowStatus;

        fn release(pc: Pin<&mut PlasmaClient>, oid: &ObjectID) -> ArrowStatus;

        fn contains(
            pc: Pin<&mut PlasmaClient>,
            oid: &ObjectID,
            has_object: &mut bool,
        ) -> ArrowStatus;

        fn abort(pc: Pin<&mut PlasmaClient>, oid: &ObjectID) -> ArrowStatus;

        fn seal(pc: Pin<&mut PlasmaClient>, oid: &ObjectID) -> ArrowStatus;

        #[cxx_name = "single_delete"]
        fn delete(pc: Pin<&mut PlasmaClient>, oid: &ObjectID) -> ArrowStatus;

        fn multi_delete(pc: Pin<&mut PlasmaClient>, oid: &CxxVector<ObjectID>) -> ArrowStatus;

        fn refresh(pc: Pin<&mut PlasmaClient>, oid: &CxxVector<ObjectID>) -> ArrowStatus;

        fn disconnect(pc: Pin<&mut PlasmaClient>) -> ArrowStatus;

        fn store_capacity_bytes(pc: Pin<&mut PlasmaClient>) -> i64;
    }
}
