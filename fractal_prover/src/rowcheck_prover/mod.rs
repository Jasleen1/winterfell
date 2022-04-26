use std::{convert::TryInto, marker::PhantomData};

use crypto::{ElementHasher, Hasher};
use fractal_utils::polynomial_utils::*;
use fri::{DefaultProverChannel, FriOptions};
use math::{FieldElement, StarkField};

use fractal_proofs::{RowcheckProof, polynom};

use crate::errors::ProverError;

pub struct RowcheckProver<B: StarkField, E: FieldElement<BaseField = B>, H: Hasher> {
    f_az_coeffs: Vec<B>,
    f_bz_coeffs: Vec<B>,
    f_cz_coeffs: Vec<B>,
    degree_fs: usize,
    size_subgroup_h: usize,
    evaluation_domain: Vec<B>,
    fri_options: FriOptions,
    num_queries: usize,
    max_degree: usize,
    eta: B,
    _h: PhantomData<H>,
    _e: PhantomData<E>,
}

impl<B: StarkField, E: FieldElement<BaseField = B>, H: ElementHasher<BaseField = B>>
    RowcheckProver<B, E, H>
{
    pub fn new(
        f_az_coeffs: Vec<B>,
        f_bz_coeffs: Vec<B>,
        f_cz_coeffs: Vec<B>,
        degree_fs: usize,
        size_subgroup_h: usize,
        evaluation_domain: Vec<B>,
        fri_options: FriOptions,
        num_queries: usize,
        max_degree: usize,
        eta: B,
    ) -> Self {
        RowcheckProver {
            f_az_coeffs,
            f_bz_coeffs,
            f_cz_coeffs,
            degree_fs,
            size_subgroup_h,
            evaluation_domain,
            fri_options,
            num_queries,
            max_degree,
            eta,
            _h: PhantomData,
            _e: PhantomData,
        }
    }

    pub fn generate_proof(&self) -> Result<RowcheckProof<B, E, H>, ProverError> {
        let mut denom_poly = vec![B::ZERO; self.size_subgroup_h];
        denom_poly.push(B::ONE);
        let h_size_32: u32 = self.size_subgroup_h.try_into().unwrap();
        let eta_pow = B::PositiveInteger::from(h_size_32);
        denom_poly[0] = self.eta.exp(eta_pow);
        let s_coeffs = polynom::div(
            &polynom::sub(&polynom::mul(&self.f_az_coeffs, &self.f_bz_coeffs), &self.f_cz_coeffs),
            &denom_poly
        );   
        let s_comp_coeffs = get_complementary_poly::<B>(self.size_subgroup_h - 2, self.max_degree - 1);
        let new_s = polynom::mul(&s_coeffs, &s_comp_coeffs);
        println!("New s deg = {}", polynom::degree_of(&new_s));
        let s_evals_b: Vec<B> = polynom::eval_many(new_s.clone().as_slice(), self.evaluation_domain.clone().as_slice());// Vec::new();
        let s_evals: Vec<E> = s_evals_b.into_iter().map(|x: B| {E::from(x)}).collect();
        // for i in 0..self.evaluation_domain.len() {
        //     let s_val_numerator =
        //         E::from(self.f_az_coeffs[i] * self.f_bz_coeffs[i] - self.f_cz_coeffs[i]);
            
        //     let s_val_denominator = E::from(compute_vanishing_poly(
        //         self.evaluation_domain[i],
        //         self.eta,
        //         self.size_subgroup_h.try_into().unwrap(),
        //     ));
        //     println!("S denom = {}, eval elt = {}", s_val_denominator, self.evaluation_domain[i]);
        //     s_evals.push(s_val_numerator / s_val_denominator);
        // }
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
        println!("Deg fs = {}", self.degree_fs);
        println!("Deg h = {}", self.size_subgroup_h);
        Ok(RowcheckProof {
            options: self.fri_options.clone(),
            num_evaluations: self.evaluation_domain.len(),
            queried_positions,
            s_proof,
            s_queried_evals,
            s_commitments,
            s_max_degree: self.size_subgroup_h - 2,
        })
    }
}
