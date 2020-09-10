pub fn uninit_vector<T>(length: usize) -> Vec<T> {
    let mut vector = Vec::with_capacity(length);
    unsafe { vector.set_len(length); }
    return vector;
}

pub fn filled_vector<T: Copy>(length: usize, capacity: usize, value: T) -> Vec<T> {
    let mut vector = vec![value; capacity];
    vector.truncate(length);
    return vector;
}

#[cfg(test)]
pub fn remove_leading_zeros(values: &[u128]) -> Vec<u128> {
    for i in (0..values.len()).rev() {
        if values[i] != 0 {
            return values[0..(i + 1)].to_vec();
        }
    }

    return [].to_vec();
}
