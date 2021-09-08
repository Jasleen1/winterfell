mod tests;

pub use std::convert::TryInto;

pub use fractal_utils::{errors::MatrixError, matrix_utils::*, polynomial_utils::*, *};
pub use winter-fri::{FriOptions, FriProof, DefaultProverChannel, PublicCoin};
pub use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
    utils,
};
pub struct RowcheckProof<E: FieldElement + From<BaseElement>> {
    pub options: FriOptions,
    pub num_evaluations: usize,
    pub queried_positions: Vec<usize>,
    pub s_proof: FriProof,
    pub s_queried_evals: Vec<E>,
    pub s_commitments: Vec<[u8; 32]>,
    pub s_max_degree: usize,
}

pub struct SumcheckProof<E: FieldElement + From<BaseElement>> {
    pub options: FriOptions,
    pub num_evaluations: usize,
    // Question: is it ok to use the same queried positions for both
    // g and e of different degrees?
    pub queried_positions: Vec<usize>,
    pub g_proof: FriProof,
    pub g_queried_evals: Vec<E>,
    pub g_commitments: Vec<[u8; 32]>,
    pub g_max_degree: usize,
    pub e_proof: FriProof,
    pub e_queried_evals: Vec<E>,
    pub e_commitments: Vec<[u8; 32]>,
    pub e_max_degree: usize,
}


pub struct LincheckProof<E: FieldElement + From<BaseElement>> {
    pub options: FriOptions,
    pub num_evaluations: usize,
    // Question: is it ok to use the same queried positions for both
    // g and e of different degrees?
    pub queried_positions: Vec<usize>,
    pub g_proof: FriProof,
    pub g_queried_evals: Vec<E>,
    pub g_commitments: Vec<[u8; 32]>,
    pub g_max_degree: usize,
    pub e_proof: FriProof,
    pub e_queried_evals: Vec<E>,
    pub e_commitments: Vec<[u8; 32]>,
    pub e_max_degree: usize,
}


pub struct MatrixArithProof<E: FieldElement + From<BaseElement>> {
    pub options: FriOptions,
    pub num_evaluations: usize,
    pub proof_of_val: SumcheckProof<E>,
}
