use std::convert::TryInto;

use fractal_utils::{errors::MatrixError, matrix_utils::*, polynomial_utils::*, *};
use fri::{FriOptions, FriProof, DefaultProverChannel, PublicCoin};
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
    utils,
};

use fractal_proofs::RowcheckProof;

pub struct RowcheckProver<E: FieldElement + From<BaseElement>> {
    f_az_evals: Vec<E>, 
    f_bz_evals: Vec<E>,
    f_cz_evals: Vec<E>,
    degree_fs: usize,
    size_subgroup_h: usize,
    evaluation_domain: Vec<BaseElement>,
    fri_options: FriOptions,
    num_queries: usize,
}

impl<E: FieldElement + From<BaseElement> + StarkField> RowcheckProver<E> {
    pub fn new(
        f_az_evals: Vec<E>, 
        f_bz_evals: Vec<E>,
        f_cz_evals: Vec<E>,
        degree_fs: usize,
        size_subgroup_h: usize,
        evaluation_domain: Vec<BaseElement>,
        fri_options: FriOptions,
        num_queries: usize,
    ) -> Self {
        RowcheckProver{
            f_az_evals, 
            f_bz_evals,
            f_cz_evals,
            degree_fs,
            size_subgroup_h,
            evaluation_domain,
            fri_options,
            num_queries,
        }
    } 

    pub fn generate_proof(&self) -> RowcheckProof<E> {
        let mut s_evals: Vec<E>  = Vec::new();
        for i in 0..self.evaluation_domain.len() {
            let s_val_numerator = self.f_az_evals[i] * self.f_bz_evals[i] - self.f_cz_evals[i];
            let s_val_denominator = E::from(vanishing_poly_for_mult_subgroup(self.evaluation_domain[i], self.size_subgroup_h.try_into().unwrap()));
            s_evals[i] = s_val_numerator / s_val_denominator;// TODO divide by v_H(X)
        }
        let mut fri_prover = fri::FriProver::new(self.fri_options.clone());
        let mut channel = DefaultProverChannel::new(
            self.fri_options.clone(),
            self.evaluation_domain.len(),
            self.num_queries,
        );
        let query_positions = channel.draw_query_positions();
        let queried_positions = query_positions.clone();
        // Build proofs for the polynomial g
        fri_prover.build_layers(
            &mut channel,
            s_evals.clone(),
            &self.evaluation_domain,
        );
        let s_proof = fri_prover.build_proof(&query_positions);
        let s_queried_evals = query_positions
        .iter()
        .map(|&p| s_evals[p])
        .collect::<Vec<_>>();
        let s_commitments = channel.fri_layer_commitments().to_vec();
        RowcheckProof{
            options: self.fri_options.clone(),
            num_evaluations: self.evaluation_domain.len(),
            queried_positions,
            s_proof,
            s_queried_evals,
            s_commitments,
            s_max_degree: self.degree_fs - 1,
        }
    }
}