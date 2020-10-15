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

// MATH
// ================================================================================================

pub fn log2(x: usize) -> u32 {
    assert!(x.is_power_of_two(), "x must be a power of two");
    let num_bits = (std::mem::size_of::<usize>() * 8) as u32;
    num_bits - x.leading_zeros() - 1
}
