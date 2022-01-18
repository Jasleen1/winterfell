use fractal_proofs::{FriOptions, StarkField};

mod errors;
mod lincheck_prover;
pub mod prover;
mod rowcheck_prover;
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct FractalOptions<B: StarkField> {
    degree_fs: usize,
    size_subgroup_h: u128,
    size_subgroup_k: u128,
    summing_domain: Vec<B>,
    evaluation_domain: Vec<B>,
    h_domain: Vec<B>,
    fri_options: FriOptions,
    num_queries: usize,
}
