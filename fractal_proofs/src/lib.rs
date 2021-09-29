mod tests;

pub use std::convert::TryInto;

use crypto::Hasher;
pub use fractal_utils::{errors::MatrixError, matrix_utils::*, polynomial_utils::*, *};
pub use fri::{DefaultProverChannel, FriOptions, FriProof};
pub use math::{fft, fields::f128::BaseElement, FieldElement, StarkField, *};
pub use winter_utils::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
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

impl<B: StarkField, E: FieldElement<BaseField = B>, H: Hasher> Serializable
    for RowcheckProof<B, E, H>
{
    /// Serializes `self` and writes the resulting bytes into the `target` writer.
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u8(self.num_evaluations as u8);
        target.write_u8(self.queried_positions.len() as u8);
        for pos in 0..self.queried_positions.len() {
            target.write_u8(self.queried_positions[pos] as u8);
        }
        self.s_proof.write_into(target);
        self.s_queried_evals.write_into(target);
        self.s_commitments.write_into(target);
        target.write_u8(self.s_max_degree as u8);
    }
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

impl<B: StarkField, E: FieldElement<BaseField = B>, H: Hasher> Serializable
    for SumcheckProof<B, E, H>
{
    /// Serializes `self` and writes the resulting bytes into the `target` writer.
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u8(self.num_evaluations as u8);
        target.write_u8(self.queried_positions.len() as u8);
        for pos in 0..self.queried_positions.len() {
            target.write_u8(self.queried_positions[pos] as u8);
        }
        self.g_proof.write_into(target);
        self.g_queried_evals.write_into(target);
        self.g_commitments.write_into(target);
        target.write_u8(self.g_max_degree as u8);

        self.e_proof.write_into(target);
        self.e_queried_evals.write_into(target);
        self.e_commitments.write_into(target);
        target.write_u8(self.e_max_degree as u8);
    }
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
