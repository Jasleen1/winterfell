use crate::field::{FieldElement, StarkField};

// VECTOR FUNCTIONS
// ================================================================================================

pub fn uninit_vector<T>(length: usize) -> Vec<T> {
    let mut vector = Vec::with_capacity(length);
    unsafe {
        vector.set_len(length);
    }
    vector
}

pub fn filled_vector<T: Copy>(length: usize, capacity: usize, value: T) -> Vec<T> {
    let mut vector = vec![value; capacity];
    vector.truncate(length);
    vector
}

pub fn remove_leading_zeros(values: &[FieldElement]) -> Vec<FieldElement> {
    for i in (0..values.len()).rev() {
        if values[i] != FieldElement::ZERO {
            return values[..(i + 1)].to_vec();
        }
    }

    [].to_vec()
}
