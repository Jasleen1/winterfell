use sha3::Digest;

/// Wrapper around blake3 hash function
pub fn blake3(values: &[u8], result: &mut [u8]) {
    debug_assert!(
        result.len() == 32,
        "expected result to be exactly 32 bytes but received {}",
        result.len()
    );
    let hash = blake3::hash(&values);
    result.copy_from_slice(hash.as_bytes());
}

/// Wrapper around sha3 hash function
pub fn sha3(values: &[u8], result: &mut [u8]) {
    debug_assert!(
        result.len() == 32,
        "expected result to be exactly 32 bytes but received {}",
        result.len()
    );
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(&values);
    let hash = hasher.finalize();
    result.copy_from_slice(hash.as_ref());
}
