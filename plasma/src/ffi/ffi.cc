#include "src/ffi/ffi.h"
#include "plasma/src/ffi/mod.rs.h"

namespace plasma {

  //////////////
  // ObjectID //
  //////////////

  std::unique_ptr<ObjectID> oid_from_binary(rust::Slice<const uint8_t> binary) {
    std::string bin_str = std::string(reinterpret_cast<const char*>(binary.data()), binary.size());
    ObjectID oid = plasma::ObjectID::from_binary(bin_str);
    return std::make_unique<ObjectID>(oid);
  }

  rust::Slice<const uint8_t> oid_to_binary(const ObjectID& oid) {
    std::string bin = oid.binary();
    uint8_t const *c = reinterpret_cast<const uint8_t*>(bin.data());
    return rust::Slice<const uint8_t>(c, sizeof(c));
  }

  rust::String oid_to_hex(const ObjectID& oid) {
    std::string hex = oid.hex();
    return rust::String(hex);
  }

  int64_t oid_size(const ObjectID& oid){
    return oid.size();
  }

  bool oid_equals(const ObjectID& oid1, const ObjectID& oid2){
    return oid1 == oid2;
  }

  ////////////
  // Buffer //
  ////////////

  std::unique_ptr<ObjectBuffer> new_obj_buffer() {
    std::shared_ptr<Buffer> data_ptr;
    std::shared_ptr<Buffer> metadata_ptr;
    return std::make_unique<ObjectBuffer>(ObjectBuffer{data_ptr, metadata_ptr, 0});
  }

  rust::Slice<const unsigned char> get_buffer_data(std::shared_ptr<Buffer> buffer) {
    const uint8_t *c = buffer->data();
    int64_t len = buffer->size();
    return rust::Slice<const unsigned char>(c, len);
  }

  rust::Slice<unsigned char> get_buffer_data_mut(std::shared_ptr<Buffer> buffer) {
    uint8_t *c = buffer->mutable_data();
    int64_t len = buffer->size();
    return rust::Slice<unsigned char>(c, len);
  }

  //////////////////
  // PlasmaClient //
  //////////////////

  std::unique_ptr<PlasmaClient> new_plasma_client() {
    return std::make_unique<PlasmaClient>(plasma::PlasmaClient());
  }

  ArrowStatus connect(PlasmaClient& pc, rust::Str store_socket_name, uint32_t num_retries) {
    std::string manager_socket("");
    Status conn_status = pc.Connect(std::string(store_socket_name), manager_socket, 0, num_retries);
    return ArrowStatus{make_plasma_error(conn_status.code()), conn_status.message()};
  }

  ArrowStatus set_client_options(PlasmaClient& pc, rust::Str client_name, int64_t output_memory_quota){
    Status client_status = pc.SetClientOptions(std::string(client_name), output_memory_quota);
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus create(PlasmaClient& pc, ObjectBuffer& ob, const ObjectID& oid, int64_t data_size, rust::Slice<const uint8_t> metadata){
    std::shared_ptr<Buffer>* data_ptr = &ob.data;
    Status client_status = pc.Create(oid, data_size, metadata.data(), metadata.size(), data_ptr, 0, true);
    ob.metadata = std::make_shared<Buffer>(metadata.data(), metadata.size());
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus create_and_seal(PlasmaClient& pc, const ObjectID& oid, rust::Slice<const uint8_t> data, rust::Slice<const uint8_t> metadata){
    std::string bin_data = std::string(reinterpret_cast<const char*>(data.data()), data.size());
    std::string bin_metadata = std::string(reinterpret_cast<const char*>(metadata.data()), metadata.size());

    Status client_status = pc.CreateAndSeal(oid, bin_data, bin_metadata, true);
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus get(PlasmaClient& pc, const ObjectID& oid, int64_t timeout_ms, ObjectBuffer& ob) {
    const ObjectID* oidp = &oid;
    ObjectBuffer* obp = &ob;
    Status client_status = pc.Get(oidp, 1, timeout_ms, obp);
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus multi_get(PlasmaClient& pc, const std::vector<ObjectID>& oids, int64_t timeout_ms, std::vector<ObjectBuffer>& obs){
    std::vector<ObjectBuffer>* buffers = &obs;
    Status client_status = pc.Get(oids, timeout_ms, buffers);
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus release(PlasmaClient& pc, const ObjectID& oid) {
    Status client_status = pc.Release(oid);
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus contains(PlasmaClient& pc, const ObjectID& oid, bool& has_object){
    bool* has_res = &has_object;
    Status client_status = pc.Contains(oid, has_res);
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus abort(PlasmaClient& pc, const ObjectID& oid){
    Status client_status = pc.Abort(oid);
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus seal(PlasmaClient& pc, const ObjectID& oid) {
    Status client_status = pc.Seal(oid);
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus single_delete(PlasmaClient& pc, const ObjectID& oid) {
    Status client_status = pc.Delete(oid);
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus multi_delete(PlasmaClient& pc, const std::vector<ObjectID>& oids) {
    Status client_status = pc.Delete(oids);
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus refresh(PlasmaClient& pc, const std::vector<ObjectID>& oids) {
    Status client_status = pc.Refresh(oids);
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  ArrowStatus disconnect(PlasmaClient& pc) {
    Status client_status = pc.Disconnect();
    return ArrowStatus{make_plasma_error(client_status.code()), client_status.message()};
  }

  int64_t store_capacity_bytes(PlasmaClient& pc) {
    return pc.store_capacity();
  }

  ///////////
  // utils //
  ///////////

  StatusCode make_plasma_error(arrow::StatusCode code) {
    StatusCode plasma_code = StatusCode::UnknownError;
    switch (code) {
    case arrow::StatusCode::OK:
      plasma_code = StatusCode::OK;
      break;
    case arrow::StatusCode::OutOfMemory:
      plasma_code = StatusCode::OutOfMemory;
      break;
    case arrow::StatusCode::KeyError:
      plasma_code = StatusCode::KeyError;
      break;
    case arrow::StatusCode::TypeError:
      plasma_code = StatusCode::TypeError;
      break;
    case arrow::StatusCode::Invalid:
      plasma_code = StatusCode::Invalid;
      break;
    case arrow::StatusCode::IOError:
      plasma_code = StatusCode::IOError;
      break;
    case arrow::StatusCode::CapacityError:
      plasma_code = StatusCode::CapacityError;
      break;
    case arrow::StatusCode::IndexError:
      plasma_code = StatusCode::IndexError;
      break;
    case arrow::StatusCode::UnknownError:
      plasma_code = StatusCode::UnknownError;
      break;
    case arrow::StatusCode::NotImplemented:
      plasma_code = StatusCode::NotImplemented;
      break;
    case arrow::StatusCode::SerializationError:
      plasma_code = StatusCode::SerializationError;
      break;
    case arrow::StatusCode::RError:
      plasma_code = StatusCode::RError;
      break;
    case arrow::StatusCode::CodeGenError:
      plasma_code = StatusCode::CodeGenError;
      break;
    case arrow::StatusCode::ExpressionValidationError:
      plasma_code = StatusCode::ExpressionValidationError;
      break;
    case arrow::StatusCode::ExecutionError:
      plasma_code = StatusCode::ExecutionError;
      break;
    case arrow::StatusCode::AlreadyExists:
      plasma_code = StatusCode::AlreadyExists;
      break;
    }
    return plasma_code;
  }


}
