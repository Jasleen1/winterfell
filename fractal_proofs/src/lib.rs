mod tests;

pub use std::convert::TryInto;

use crypto::Hasher;
pub use fractal_utils::{errors::MatrixError, matrix_utils::*, polynomial_utils::*, *};
pub use fri::{FriOptions, FriProof, DefaultProverChannel};
pub use math::{
    fft,
    FieldElement, StarkField,
    fields::f128::BaseElement,
    utils,
};
pub struct RowcheckProof<B: StarkField, E: FieldElement<BaseField = B>, H: Hasher> {
    pub options: FriOptions,
    pub num_evaluations: usize,
    pub queried_positions: Vec<usize>,
    pub s_proof: FriProof,
    pub s_queried_evals: Vec<E>,
    pub s_commitments: Vec<<H>::Digest>,
    pub s_max_degree: usize,
}

pub struct SumcheckProof<B: StarkField, E: FieldElement<BaseField = B>, H: Hasher> {
    pub options: FriOptions,
    pub num_evaluations: usize,
    // Question: is it ok to use the same queried positions for both
    // g and e of different degrees?
    pub queried_positions: Vec<usize>,
    pub g_proof: FriProof,
    pub g_queried_evals: Vec<E>,
    pub g_commitments: Vec<<H>::Digest>,
    pub g_max_degree: usize,
    pub e_proof: FriProof,
    pub e_queried_evals: Vec<E>,
    pub e_commitments: Vec<<H>::Digest>,
    pub e_max_degree: usize,
}


pub struct LincheckProof<B: StarkField, E: FieldElement<BaseField = B>, H: Hasher> {
    pub options: FriOptions,
    pub num_evaluations: usize,
    // Question: is it ok to use the same queried positions for both
    // g and e of different degrees?
    pub queried_positions: Vec<usize>,
    pub g_proof: FriProof,
    pub g_queried_evals: Vec<E>,
    pub g_commitments: Vec<<H>::Digest>,
    pub g_max_degree: usize,
    pub e_proof: FriProof,
    pub e_queried_evals: Vec<E>,
    pub e_commitments: Vec<<H>::Digest>,
    pub e_max_degree: usize,
}


pub struct MatrixArithProof<B: StarkField, E: FieldElement<BaseField = B>, H: Hasher> {
    pub options: FriOptions,
    pub num_evaluations: usize,
    pub proof_of_val: SumcheckProof<B, E, H>,
}
