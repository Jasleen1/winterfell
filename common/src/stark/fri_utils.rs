use crate::utils::uninit_vector;
use crypto::HashFunction;
use math::field::{AsBytes, FieldElement};

pub fn get_augmented_positions(positions: &[usize], column_length: usize) -> Vec<usize> {
    let row_length = column_length / 4;
    let mut result = Vec::new();
    for position in positions {
        let ap = position % row_length;
        if !result.contains(&ap) {
            result.push(ap);
        }
    }
    result
}

pub fn hash_values(values: &[[FieldElement; 4]], hash: HashFunction) -> Vec<[u8; 32]> {
    let mut result: Vec<[u8; 32]> = uninit_vector(values.len());
    for i in 0..values.len() {
        hash((&values[i]).as_bytes(), &mut result[i]);
    }
    result
}
