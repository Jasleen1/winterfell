use std::{convert::TryInto, marker::PhantomData};

use crypto::{ElementHasher, Hasher};
use fractal_utils::polynomial_utils::*;
use fri::{DefaultProverChannel, FriOptions};
use math::{FieldElement, StarkField};

use fractal_proofs::RowcheckProof;

use crate::errors::ProverError;

pub struct RowcheckProver<B: StarkField, E: FieldElement<BaseField = B>, H: Hasher> {
    f_az_evals: Vec<B>,
    f_bz_evals: Vec<B>,
    f_cz_evals: Vec<B>,
    degree_fs: usize,
    size_subgroup_h: usize,
    evaluation_domain: Vec<B>,
    fri_options: FriOptions,
    num_queries: usize,
    _h: PhantomData<H>,
    _e: PhantomData<E>,
}

impl<B: StarkField, E: FieldElement<BaseField = B>, H: ElementHasher<BaseField = B>>
    RowcheckProver<B, E, H>
{
    pub fn new(
        f_az_evals: Vec<B>,
        f_bz_evals: Vec<B>,
        f_cz_evals: Vec<B>,
        degree_fs: usize,
        size_subgroup_h: usize,
        evaluation_domain: Vec<B>,
        fri_options: FriOptions,
        num_queries: usize,
    ) -> Self {
        RowcheckProver {
            f_az_evals,
            f_bz_evals,
            f_cz_evals,
            degree_fs,
            size_subgroup_h,
            evaluation_domain,
            fri_options,
            num_queries,
            _h: PhantomData,
            _e: PhantomData,
        }
    }

    pub fn generate_proof(&self) -> Result<RowcheckProof<B, E, H>, ProverError> {
        let mut s_evals: Vec<E> = Vec::new();
        for i in 0..self.evaluation_domain.len() {
            let s_val_numerator =
                E::from(self.f_az_evals[i] * self.f_bz_evals[i] - self.f_cz_evals[i]);
            let s_val_denominator = E::from(vanishing_poly_for_mult_subgroup(
                self.evaluation_domain[i],
                self.size_subgroup_h.try_into().unwrap(),
            ));
            s_evals[i] = s_val_numerator / s_val_denominator;
        }
        let mut channel = DefaultProverChannel::new(self.evaluation_domain.len(), self.num_queries);
        let mut fri_prover =
            fri::FriProver::<B, E, DefaultProverChannel<B, E, H>, H>::new(self.fri_options.clone());

        let query_positions = channel.draw_query_positions();
        let queried_positions = query_positions.clone();
        // Build proofs for the polynomial g
        fri_prover.build_layers(&mut channel, s_evals.clone());
        let s_proof = fri_prover.build_proof(&query_positions);
        let s_queried_evals = query_positions
            .iter()
            .map(|&p| s_evals[p])
            .collect::<Vec<_>>();
        let s_commitments = channel.layer_commitments().to_vec();
        Ok(RowcheckProof {
            options: self.fri_options.clone(),
            num_evaluations: self.evaluation_domain.len(),
            queried_positions,
            s_proof,
            s_queried_evals,
            s_commitments,
            s_max_degree: self.degree_fs - 1,
        })
    }
}
