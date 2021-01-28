/// Converts a list of 20-byte arrays into plasma store object IDs.
pub fn map_object_ids(object_ids: &[crate::ObjectId]) -> Vec<plasma::ObjectId> {
    object_ids
        .iter()
        .map(|oid| plasma::ObjectId::new(*oid))
        .collect()
}
