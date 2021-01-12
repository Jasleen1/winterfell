use cxx::UniquePtr;
use rand::Rng;
use std::fmt;

mod ffi;
use ffi::ffi as plasma;

#[cfg(test)]
mod tests;

// OBJECT ID
// ================================================================================================

pub struct ObjectId(UniquePtr<plasma::ObjectID>);

impl ObjectId {
    /// Returns a new object ID instantiated from the specified bytes.
    pub fn new(bytes: [u8; 20]) -> Self {
        ObjectId(plasma::oid_from_binary(&bytes))
    }

    /// Returns a new object ID instantiated from a random sequence of 20 bytes.
    pub fn rand() -> Self {
        Self::new(rand::thread_rng().gen())
    }

    /// Returns binary representation of the object ID.
    pub fn to_bytes(&self) -> &[u8] {
        plasma::oid_to_binary(&self.0)
    }

    /// Returns hexadecimal representation fo the object ID.
    pub fn to_hex(&self) -> String {
        plasma::oid_to_hex(&self.0)
    }

    fn inner(&self) -> &UniquePtr<plasma::ObjectID> {
        &self.0
    }
}

impl Clone for ObjectId {
    fn clone(&self) -> Self {
        ObjectId(plasma::oid_from_binary(&self.to_bytes()))
    }
}

impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl PartialEq for ObjectId {
    fn eq(&self, other: &Self) -> bool {
        plasma::oid_equals(&self.0, &other.0)
    }
}

// OBJECT BUFFER
// ================================================================================================

pub struct ObjectBuffer {
    id: ObjectId,
    buf: UniquePtr<plasma::ObjectBuffer>,
}

impl ObjectBuffer {

    fn new(id: ObjectId, buf: UniquePtr<plasma::ObjectBuffer>) -> Self {
        ObjectBuffer {id, buf}
    }

    /// Returns object ID of this object buffer.
    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    /// Returns read-only data buffer of this object buffer.
    pub fn data(&self) -> &[u8] {
        plasma::get_buffer_data(self.buf.data.clone())
    }

    /// Returns mutable data buffer of this object buffer.
    pub fn data_mut(&mut self) -> &mut [u8] {
        plasma::get_buffer_data_mut(self.buf.data.clone())
    }

    /// Returns metadata buffer of this object buffer.
    pub fn meta(&self) -> &[u8] {
        plasma::get_buffer_data(self.buf.metadata.clone())
    }

    /// Returns true if the data buffer of this object buffer is mutable.
    pub fn is_mutable(&self) -> bool {
        plasma::is_buffer_mutable(self.buf.data.clone())
    }
}

impl fmt::Debug for ObjectBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(id: {}, size: {})", self.id.to_hex(), self.data().len())
    }
}

impl fmt::Display for ObjectBuffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(id: {}, size: {})", self.id.to_hex(), self.data().len())
    }
}

// PLASMA CLIENT
// ================================================================================================

pub struct PlasmaClient(UniquePtr<plasma::PlasmaClient>);

impl PlasmaClient {
    pub fn new() -> Self {
        PlasmaClient(plasma::new_plasma_client())
    }

    /// Connect this client to the local plasma store.
    /// * `store_socket_name` The name of the UNIX domain socket to use to
    ///        connect to the Plasma store.
    /// * `num_retries` number of attempts to connect to IPC socket, default 50
    pub fn connect(&mut self, store_socket_name: &str, num_retries: u32) {
        let _result = plasma::connect(self.0.pin_mut(), store_socket_name, num_retries);
    }

    /// Retrieves an object with the specified ID from the store. This function will block until
    /// the object has been created and sealed in the Plasma store or the timeout expires.
    /// * `oid` The ID of the object to get.
    /// * `timeout_ms` The amount of time in milliseconds to wait before this
    ///        request times out. If this value is -1, then no timeout is set.
    /// The caller is responsible for releasing any retrieved objects, but it
    /// should not release objects that were not retrieved.
    pub fn get(&mut self, oid: ObjectId, timeout_ms: i64) -> ObjectBuffer {
        let mut ob = plasma::new_obj_buffer();
        let _result = plasma::get(self.0.pin_mut(), oid.inner(), timeout_ms, ob.pin_mut());
        ObjectBuffer::new(oid, ob)
    }

    /// Create an object in the Plasma Store. Any metadata for this object must be
    /// be passed in when the object is created.
    /// * `oid` The ID to use for the newly crated object.
    /// * `data_size` The size in bytes of the space to be allocated for this object's data
    ///     (this does not included space used for metadata).
    /// * `meta` The object's metadata; if there is no metadata, this should be an empty slice.
    ///
    /// The returned object must be released once it is done with. It must also
    /// be either sealed or aborted.
    pub fn create(&mut self, oid: ObjectId, data_size: usize, meta: &[u8]) -> ObjectBuffer {
        let mut ob = plasma::new_obj_buffer();
        let _result = plasma::create(
                self.0.pin_mut(),
                ob.pin_mut(),
                oid.inner(),
                data_size as i64,
                meta,
            );
        ObjectBuffer::new(oid, ob)
    }

    /// Tells the store that the client no longer needs the object. This should be
    /// called after Get() or Create() when the client is done with the object.
    /// After this call, the buffer returned by Get() is no longer valid.
    pub fn release(&mut self, ob: ObjectBuffer) {
        let _result = plasma::release(self.0.pin_mut(), ob.id().inner());
    }

    /// Create and seal an object in the object store. This is an optimization
    /// which allows small objects to be created quickly with fewer messages to
    /// the store.
    /// * `oid` The ID for the object to create.
    /// * `data` The data for the object to create.
    /// * `meta` The metadata for the object to create.
    pub fn create_and_seal(&mut self, oid: ObjectId, data: &[u8], meta: &[u8]) {
        let _result = plasma::create_and_seal(self.0.pin_mut(), oid.inner(), data, meta);
    }

    /// Abort an unsealed object in the object store. If the abort succeeds, then
    /// it will be as if the object was never created at all.
    pub fn abort(&mut self, ob: ObjectBuffer) {
        let _result = plasma::abort(self.0.pin_mut(), ob.id().inner());
    }

    /// Seal an object in the object store. The object will be immutable after this call.
    pub fn seal(&mut self, ob: &ObjectBuffer) {
        let _result = plasma::seal(self.0.pin_mut(), ob.id().inner());
    }

    /// Delete an object from the object store. This currently assumes that the
    /// object is present, has been sealed and not used by another client. Otherwise,
    /// it is a no operation.
    pub fn delete(&mut self, oid: ObjectId) {
        let _result = plasma::delete(self.0.pin_mut(), oid.inner());
    }

    /// Check if the object store contains a particular object and the object has
    /// been sealed.
    pub fn contains(&mut self, oid: ObjectId) -> bool {
        let mut has_object = false;
        let _result = plasma::contains(self.0.pin_mut(), oid.inner(), &mut has_object);
        has_object
    }
}
