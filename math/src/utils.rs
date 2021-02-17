use crate::field::FieldElement;

// VECTOR FUNCTIONS
// ================================================================================================

pub fn uninit_vector<T>(length: usize) -> Vec<T> {
    let mut vector = Vec::with_capacity(length);
    unsafe {
        vector.set_len(length);
    }
    vector
}

pub fn remove_leading_zeros<E: FieldElement>(values: &[E]) -> Vec<E> {
    for i in (0..values.len()).rev() {
        if values[i] != E::ZERO {
            return values[..(i + 1)].to_vec();
        }
    }

    [].to_vec()
}
