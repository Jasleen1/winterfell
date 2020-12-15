use crypto::HashFunction;
use math::field::FieldElement;

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

pub fn hash_values<E: FieldElement>(values: &[[E; 4]], hash: HashFunction) -> Vec<[u8; 32]> {
    let mut result: Vec<[u8; 32]> = uninit_vector(values.len());
    // TODO: ideally, this should be done using something like update() method of a hasher
    let mut buf = vec![0u8; 4 * E::ELEMENT_BYTES];
    for i in 0..values.len() {
        buf[..E::ELEMENT_BYTES].copy_from_slice(&values[i][0].to_bytes());
        buf[E::ELEMENT_BYTES..E::ELEMENT_BYTES * 2].copy_from_slice(&values[i][1].to_bytes());
        buf[E::ELEMENT_BYTES * 2..E::ELEMENT_BYTES * 3].copy_from_slice(&values[i][2].to_bytes());
        buf[E::ELEMENT_BYTES * 3..E::ELEMENT_BYTES * 4].copy_from_slice(&values[i][3].to_bytes());
        hash(&buf, &mut result[i]);
    }
    result
}

fn uninit_vector<T>(length: usize) -> Vec<T> {
    let mut vector = Vec::with_capacity(length);
    unsafe {
        vector.set_len(length);
    }
    vector
}
