use cxx::UniquePtr;
use rand::Rng;

mod ffi;
use ffi::ffi as plasma;

#[cfg(test)]
mod tests;

// OBJECT ID
// ================================================================================================

#[derive(Clone, Debug)]
pub struct ObjectId([u8; 20]);

impl ObjectId {
    pub fn rand() -> Self {
        ObjectId(rand::thread_rng().gen())
    }

    fn ptr(&self) -> UniquePtr<plasma::ObjectID> {
        unsafe { plasma::oid_from_binary(&self.0) }
    }
}

// OBJECT BUFFER
// ================================================================================================

pub struct ObjectBuffer(UniquePtr<plasma::ObjectBuffer>);

impl ObjectBuffer {
    pub fn data(&self) -> &[u8] {
        unsafe { plasma::get_buffer_data(self.0.data.clone()) }
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        unsafe { plasma::get_buffer_data_mut(self.0.data.clone()) }
    }

    pub fn meta(&self) -> &[u8] {
        unsafe { plasma::get_buffer_data(self.0.metadata.clone()) }
    }
}

// PLASMA CLIENT
// ================================================================================================

pub struct PlasmaClient(UniquePtr<plasma::PlasmaClient>);

impl PlasmaClient {
    pub fn new() -> Self {
        let ptr: UniquePtr<plasma::PlasmaClient> = unsafe { plasma::new_plasma_client() };
        PlasmaClient(ptr)
    }

    pub fn connect(&mut self, store_socket_name: &str, num_retries: u32) {
        let _result = unsafe { plasma::connect(self.0.pin_mut(), store_socket_name, num_retries) };
    }

    /// Retrieves an object with the specified object ID from the store.
    pub fn get(&mut self, oid: &ObjectId, timeout_ms: i64) -> ObjectBuffer {
        let mut ob = unsafe { plasma::new_obj_buffer() };
        let _result =
            unsafe { plasma::get(self.0.pin_mut(), &oid.ptr(), timeout_ms, ob.pin_mut()) };
        ObjectBuffer(ob)
    }

    pub fn create_and_seal(&mut self, oid: &ObjectId, data: &[u8], meta: &[u8]) {
        let _result = unsafe { plasma::create_and_seal(self.0.pin_mut(), &oid.ptr(), data, meta) };
    }

    pub fn create(&mut self, oid: &ObjectId, data_size: usize, meta: &[u8]) -> ObjectBuffer {
        let mut ob = unsafe { plasma::new_obj_buffer() };
        let _result = unsafe {
            plasma::create(
                self.0.pin_mut(),
                ob.pin_mut(),
                &oid.ptr(),
                data_size as i64,
                meta,
            )
        };
        ObjectBuffer(ob)
    }

    pub fn abort(&mut self, oid: &ObjectId) {
        let _result = unsafe { plasma::abort(self.0.pin_mut(), &oid.ptr()) };
    }

    pub fn seal(&mut self, oid: &ObjectId) {
        let _result = unsafe { plasma::seal(self.0.pin_mut(), &oid.ptr()) };
    }
}
