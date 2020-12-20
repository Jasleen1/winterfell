use crypto::HashFunction;
use math::field::FieldElement;

/// Maps positions in the current evaluation domain, to positions in the folded domain.
pub fn fold_positions(
    positions: &[usize],
    source_domain_size: usize,
    folding_factor: usize,
) -> Vec<usize> {
    let target_domain_size = source_domain_size / folding_factor;

    let mut result = Vec::new();
    for position in positions {
        let position = position % target_domain_size;
        // make sure we don't record duplicated values
        if !result.contains(&position) {
            result.push(position);
        }
    }

    result
}

/// Maps positions in the evaluation domain to indexes of commitment Merkle tree.
pub fn map_positions_to_indexes(
    positions: &[usize],
    source_domain_size: usize,
    folding_factor: usize,
    num_partitions: usize,
) -> Vec<usize> {
    // if there was only 1 partition, order of elements in the commitment tree
    // is the same as the order of elements in the evaluation domain
    if num_partitions == 1 {
        return positions.to_vec();
    }

    let target_domain_size = source_domain_size / folding_factor;
    let partition_size = target_domain_size / num_partitions;

    let mut result = Vec::new();
    for position in positions {
        let partition_idx = position % num_partitions;
        let local_idx = (position - partition_idx) / num_partitions;
        let position = partition_idx * partition_size + local_idx;
        result.push(position);
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

pub fn uninit_vector<T>(length: usize) -> Vec<T> {
    let mut vector = Vec::with_capacity(length);
    unsafe {
        vector.set_len(length);
    }
    vector
}
