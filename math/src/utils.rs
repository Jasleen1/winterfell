#[cfg(test)]
pub fn remove_leading_zeros(values: &[u128]) -> Vec<u128> {
    for i in (0..values.len()).rev() {
        if values[i] != 0 {
            return values[0..(i + 1)].to_vec();
        }
    }

    [].to_vec()
}
