use super::ProofOptions;
use rand::distributions::Uniform;
use rand::prelude::*;

// Pseudo-randomly generates a sequence of indexes into the LDE domain
pub fn compute_trace_query_positions(
    seed: [u8; 32],
    lde_domain_size: usize,
    options: &ProofOptions,
) -> Vec<usize> {
    let num_queries = options.num_queries();

    // use the seed to construct a PRNG
    let range = Uniform::from(0..lde_domain_size);
    let mut index_iter = StdRng::from_seed(seed).sample_iter(range);

    // draw values from PRNG until we get as many unique values as specified by
    // num_queries, but skipping values which are a multiple of blowup factor
    let mut result = Vec::new();
    for _ in 0..1000 {
        let value = index_iter.next().unwrap();
        if value % options.blowup_factor() == 0 {
            continue;
        }
        if result.contains(&value) {
            continue;
        }
        result.push(value);
        if result.len() >= num_queries {
            break;
        }
    }

    assert!(
        result.len() == num_queries,
        "needed to generate {} query positions, but generated only {}",
        num_queries,
        result.len()
    );

    result
}
