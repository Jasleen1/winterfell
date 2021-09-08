use std::convert::TryInto;

use fractal_utils::{errors::MatrixError, matrix_utils::*, polynomial_utils::*, *};
use winter-fri::{DefaultProverChannel, FriOptions, PublicCoin};
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
};

use fractal_proofs::SumcheckProof;
#[cfg(test)]
mod tests;

pub struct SumcheckProver<E: FieldElement + From<BaseElement>> {
    summing_poly: Vec<E>,
    sigma: E,
    summing_domain: Vec<BaseElement>,
    summing_domain_twiddles: Vec<BaseElement>,
    evaluation_domain: Vec<BaseElement>,
    fri_options: FriOptions,
    num_queries: usize,
}

impl<E: FieldElement + From<BaseElement> + StarkField> SumcheckProver<E> {
    pub fn new(
        summing_poly: Vec<E>,
        sigma: E,
        summing_domain: Vec<BaseElement>,
        evaluation_domain: Vec<BaseElement>,
        fri_options: FriOptions,
        num_queries: usize,
    ) -> Self {
        let summing_domain_twiddles = fft::get_twiddles(summing_domain.len());
        SumcheckProver {
            summing_poly,
            sigma,
            summing_domain,
            summing_domain_twiddles,
            evaluation_domain,
            fri_options,
            num_queries,
        }
    }

    pub fn generate_proof(&self) -> SumcheckProof<E> {
        // compute the polynomial g such that Sigma(g, sigma) = summing_poly
        let mut fri_prover = fri::FriProver::new(self.fri_options.clone());
        let mut summing_poly_evals = self.summing_poly.clone();
        fft::evaluate_poly(
            &mut summing_poly_evals,
            &mut self.summing_domain_twiddles.clone(),
        );

        // compute the polynomial g such that Sigma(g, sigma) = summing_poly
        // compute the polynomial e such that e = (Sigma(g, sigma) - summing_poly)/v_H over the summing domain H.
        let mut g_summing_domain_evals: Vec<E> = Vec::new();
        let mut e_summing_domain_evals: Vec<E> = Vec::new();
        let _sigma_inv = self.sigma.inv();
        for i in 0..self.summing_poly.len() {
            let g_val =
                self.compute_g_poly_on_val(E::from(self.summing_domain[i]), summing_poly_evals[i]);
            g_summing_domain_evals.push(g_val);
            let e_val = self.compute_e_poly_on_val(
                E::from(self.summing_domain[i]),
                g_val,
                summing_poly_evals[i],
            );
            e_summing_domain_evals.push(e_val);
        }
        let inv_twiddles_summing_domain: Vec<BaseElement> =
            fft::get_inv_twiddles(self.summing_domain.len());
        fft::interpolate_poly(&mut g_summing_domain_evals, &inv_twiddles_summing_domain);
        fft::interpolate_poly(&mut e_summing_domain_evals, &inv_twiddles_summing_domain);

        let twiddles_evaluation_domain: Vec<BaseElement> =
            fft::get_twiddles(self.evaluation_domain.len());
        fft::evaluate_poly(&mut g_summing_domain_evals, &twiddles_evaluation_domain);
        fft::evaluate_poly(&mut e_summing_domain_evals, &twiddles_evaluation_domain);
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
            g_summing_domain_evals.clone(),
            &self.summing_domain,
        );
        let fri_proof_g = fri_prover.build_proof(&query_positions);
        let g_queried_evaluations = query_positions
            .iter()
            .map(|&p| g_summing_domain_evals[p])
            .collect::<Vec<_>>();
        let g_commitments = channel.fri_layer_commitments().to_vec();

        // reset to build proofs for the polynomial e
        fri_prover.reset();
        fri_prover.build_layers(
            &mut channel,
            e_summing_domain_evals.clone(),
            &self.summing_domain,
        );
        let fri_proof_e = fri_prover.build_proof(&query_positions);
        let e_queried_evaluations = query_positions
            .iter()
            .map(|&p| e_summing_domain_evals[p])
            .collect::<Vec<_>>();
        let e_commitments = channel.fri_layer_commitments().to_vec();

        SumcheckProof {
            options: self.fri_options.clone(),
            num_evaluations: self.evaluation_domain.len(),
            queried_positions: queried_positions,
            g_proof: fri_proof_g,
            g_queried_evals: g_queried_evaluations,
            g_commitments,
            g_max_degree: self.summing_poly.len() - 1,
            e_proof: fri_proof_e,
            e_queried_evals: e_queried_evaluations,
            e_commitments,
            e_max_degree: self.summing_poly.len() - self.summing_domain.len() + 1,
        }
    }

    pub fn compute_g_poly_on_val(&self, x_val: E, f_x_val: E) -> E {
        let dividing_factor_for_sigma: u64 = self.summing_domain.len().try_into().unwrap();
        let subtracting_factor = self.sigma * E::from(dividing_factor_for_sigma).inv();
        let _dividing_factor = x_val.inv();
        x_val * (f_x_val - subtracting_factor)
    }

    pub fn compute_sigma_function_on_val(&self, x_val: E, g_val: E) -> E {
        let dividing_factor: u64 = self.summing_domain.len().try_into().unwrap();
        x_val * g_val + (self.sigma * E::from(dividing_factor).inv())
    }

    pub fn compute_e_poly_on_val(&self, x_val: E, g_val: E, summing_poly_val: E) -> E {
        let sigma_function = self.compute_sigma_function_on_val(x_val, g_val);
        let sigma_minus_f = sigma_function - summing_poly_val;
        let vanishing_on_x =
            vanishing_poly_for_mult_subgroup(x_val, self.summing_domain.len().try_into().unwrap());
        sigma_minus_f * vanishing_on_x.inv()
    }
}
