#pragma once
#include "rust/cxx.h"
#include "arrow/cpp/src/plasma/client.h"
#include "arrow/cpp/src/plasma/common.h"
#include "plasma/src/ffi/mod.rs.h"

namespace plasma {

  //////////////
  // ObjectID //
  //////////////

  std::unique_ptr<ObjectID> oid_from_binary(rust::Slice<const uint8_t>);

  rust::Slice<const uint8_t> oid_to_binary(const ObjectID& oid);

  rust::String oid_to_hex(const ObjectID& oid);

  bool oid_equals(const ObjectID& oid1, const ObjectID& oid2);

  ////////////
  // Buffer //
  ////////////

  std::unique_ptr<ObjectBuffer> new_obj_buffer();

  rust::Slice<const unsigned char> get_buffer_data(std::shared_ptr<Buffer> buffer);
  
  rust::Slice<unsigned char> get_buffer_data_mut(std::shared_ptr<Buffer> buffer);

  //////////////////
  // PlasmaClient //
  //////////////////

  std::unique_ptr<PlasmaClient> new_plasma_client();

  ArrowStatus connect(PlasmaClient& pc, rust::Str store_socket_name, uint32_t num_retries);

  ArrowStatus set_client_options(PlasmaClient& pc, rust::Str client_name, int64_t output_memory_quota);

  ArrowStatus create(PlasmaClient& pc, ObjectBuffer& ob, const ObjectID& oid, int64_t data_size, rust::Slice<const uint8_t> metadata);

  ArrowStatus create_and_seal(PlasmaClient& pc, const ObjectID& oid, rust::Slice<const uint8_t> data, rust::Slice<const uint8_t> metadata);

  ArrowStatus get(PlasmaClient& pc, const ObjectID& oid, int64_t timeout_ms, ObjectBuffer& ob);

  ArrowStatus multi_get(PlasmaClient& pc, const std::vector<ObjectID>& oids, int64_t timeout_ms, std::vector<ObjectBuffer>& obs);

  ArrowStatus release(PlasmaClient& pc, const ObjectID& oid);

  ArrowStatus contains(PlasmaClient& pc, const ObjectID& oid, bool& has_object);

  ArrowStatus abort(PlasmaClient& pc, const ObjectID& oid);

  ArrowStatus seal(PlasmaClient& pc, const ObjectID& oid);

  ArrowStatus single_delete(PlasmaClient& pc, const ObjectID& oid);

  ArrowStatus multi_delete(PlasmaClient& pc, const std::vector<ObjectID>& oids);

  ArrowStatus refresh(PlasmaClient& pc, const std::vector<ObjectID>& oids);

  ArrowStatus disconnect(PlasmaClient& pc);

  int64_t store_capacity_bytes(PlasmaClient& pc);

  ///////////
  // utils //
  ///////////

  StatusCode make_plasma_error(arrow::StatusCode code);

}
