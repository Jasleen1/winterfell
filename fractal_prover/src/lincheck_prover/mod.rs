use std::{marker::PhantomData, usize};

use crypto::{ElementHasher, MerkleTree};
use fractal_indexer::{hash_values, snark_keys::*};
use fractal_utils::polynomial_utils::*;
use fri::ProverChannel;
use math::{FieldElement, StarkField};

use fractal_sumcheck::sumcheck_prover::*;

use fractal_proofs::{fft, polynom, LincheckProof, OracleQueries, TryInto};
use utils::transpose_slice;

use crate::{errors::LincheckError, FractalOptions};

const n: usize = 1;
// TODO: Will need to ask Irakliy whether a channel should be passed in here
pub struct LincheckProver<
    'a,
    B: StarkField,
    E: FieldElement<BaseField = B>,
    H: ElementHasher + ElementHasher<BaseField = B>,
> {
    alpha: B,
    prover_matrix_index: &'a ProverMatrixIndex<H, B>,
    f_1_poly_coeffs: Vec<B>,
    f_2_poly_coeffs: Vec<B>,
    options: &'a FractalOptions<B>,
    _h: PhantomData<H>,
    _e: PhantomData<E>,
}

impl<
        'a,
        B: StarkField,
        E: FieldElement<BaseField = B>,
        H: ElementHasher + ElementHasher<BaseField = B>,
    > LincheckProver<'a, B, E, H>
{
    pub fn new(
        alpha: B,
        prover_matrix_index: &'a ProverMatrixIndex<H, B>,
        f_1_poly_coeffs: Vec<B>,
        f_2_poly_coeffs: Vec<B>,
        options: &'a FractalOptions<B>,
    ) -> Self {
        LincheckProver {
            alpha,
            prover_matrix_index,
            f_1_poly_coeffs,
            f_2_poly_coeffs,
            options,
            _h: PhantomData,
            _e: PhantomData,
        }
    }

    pub fn generate_t_alpha_evals(&self) -> Vec<B> {
        let v_h_alpha = vanishing_poly_for_mult_subgroup(self.alpha, self.options.size_subgroup_h);
        let mut coefficient_values = Vec::new();
        for id in 0..self.options.summing_domain.len() {
            let summing_elt = self.options.summing_domain[id];
            let denom_term = self.alpha - self.prover_matrix_index.get_col_eval(summing_elt);
            let k_term_factor = denom_term.inv();
            let k_term = self.prover_matrix_index.get_val_eval(summing_elt) * k_term_factor;
            coefficient_values.push(k_term)
        }
        let mut t_evals = Vec::new();
        for x_val_id in 0..self.options.evaluation_domain.len() {
            let x_val = self.options.evaluation_domain[x_val_id];
            let v_h_x = vanishing_poly_for_mult_subgroup(x_val, self.options.size_subgroup_h);

            let mut sum_without_vs = B::ZERO;
            for id in 0..self.options.summing_domain.len() {
                let summing_elt = self.options.summing_domain[id];
                let denom_term = x_val - self.prover_matrix_index.get_row_eval(summing_elt);
                let prod_term = coefficient_values[id] * denom_term.inv();
                sum_without_vs = sum_without_vs + prod_term;
            }
            let sum_with_vs = (sum_without_vs * v_h_x) * v_h_alpha;
            t_evals.push(sum_with_vs);
        }
        t_evals
    }

    pub fn generate_t_alpha(&self, t_evals: Vec<B>) -> Vec<B> {
        let mut t_alpha_eval_domain_poly: Vec<B> = t_evals.clone();
        let twiddles_evaluation_domain: Vec<B> =
            fft::get_twiddles(self.options.evaluation_domain.len());
        fft::interpolate_poly(&mut t_alpha_eval_domain_poly, &twiddles_evaluation_domain);
        fractal_utils::polynomial_utils::get_to_degree_size(&mut t_alpha_eval_domain_poly);
        t_alpha_eval_domain_poly
    }

    pub fn generate_poly_prod(&self, t_alpha_eval_domain_poly: &Vec<B>) -> Vec<B> {
        // This function needs to compute the polynomial
        // u_H(X, alpha)*f_1 - t_alpha*f_2
        // here are the steps to this:
        // 1. find out how polynomials are represented and get u_H(X, alpha) = (X^|H| - alpha)/(X - alpha)
        // 2. Polynom includes a mul and a sub function, use these to do the respective ops
        let mut u_numerator = vec![B::ZERO; (self.options.size_subgroup_h).try_into().unwrap()];
        u_numerator[0] = self.alpha;
        u_numerator.push(B::ONE);
        let u_denominator = vec![self.alpha, B::ONE];
        let mut u_alpha = polynom::div(&u_numerator, &u_denominator);
        fractal_utils::polynomial_utils::get_to_degree_size(&mut u_alpha);
        println!("u_alpha_len = {}", u_alpha.len());
        println!("f_1_len = {}", self.f_1_poly_coeffs.len());
        println!("f_2_len = {}", self.f_2_poly_coeffs.len());
        polynom::sub(
            &polynom::mul(&u_alpha, &self.f_1_poly_coeffs),
            &polynom::mul(t_alpha_eval_domain_poly, &self.f_2_poly_coeffs),
        )
    }

    pub fn generate_lincheck_proof(&self) -> Result<LincheckProof<B, E, H>, LincheckError> {
        let t_alpha_evals = self.generate_t_alpha_evals();
        let t_alpha = self.generate_t_alpha(t_alpha_evals.clone());
        println!("t_alpha_size = {}", t_alpha.len());
        let poly_prod = self.generate_poly_prod(&t_alpha);
        // Next use poly_beta in a sumcheck proof but
        // the sumcheck domain is H, which isn't included here
        // Use that to produce the sumcheck proof.
        println!("Poly prod len = {}", poly_prod.len());
        let mut product_sumcheck_prover = SumcheckProver::<B, E, H>::new(
            poly_prod,
            vec![B::ONE],
            E::ZERO,
            self.options.h_domain.clone(),
            self.options.evaluation_domain.clone(),
            self.options.fri_options.clone(),
            self.options.num_queries,
        );
        let products_sumcheck_proof = product_sumcheck_prover.generate_proof();
        let beta =
            FieldElement::as_base_elements(&[product_sumcheck_prover.channel.draw_fri_alpha()])[0];
        let gamma = polynom::eval(&t_alpha, beta);
        let matrix_proof_numerator = polynom::mul_by_scalar(
            &self.prover_matrix_index.val_poly.polynomial,
            compute_vanishing_poly(self.alpha, B::ONE, self.options.size_subgroup_h)
                * compute_vanishing_poly(beta, B::ONE, self.options.size_subgroup_h),
        );
        let mut alpha_minus_row =
            polynom::mul_by_scalar(&self.prover_matrix_index.row_poly.polynomial, -B::ONE);
        alpha_minus_row[0] = alpha_minus_row[0] + self.alpha;
        let mut beta_minus_col =
            polynom::mul_by_scalar(&self.prover_matrix_index.col_poly.polynomial, -B::ONE);
        beta_minus_col[0] = beta_minus_col[0] + beta;
        let matrix_proof_denominator = polynom::mul(&alpha_minus_row, &beta_minus_col);
        let mut matrix_sumcheck_prover = SumcheckProver::<B, E, H>::new(
            matrix_proof_numerator,
            matrix_proof_denominator,
            E::from(gamma),
            self.options.summing_domain.clone(),
            self.options.evaluation_domain.clone(),
            self.options.fri_options.clone(),
            self.options.num_queries,
        );
        let matrix_sumcheck_proof = matrix_sumcheck_prover.generate_proof();

        let queried_positions = matrix_sumcheck_proof.queried_positions.clone();

        let row_queried_evaluations = queried_positions
            .iter()
            .map(|&p| E::from(self.prover_matrix_index.row_poly.evaluations[p]))
            .collect::<Vec<_>>();
        let row_proofs_results = queried_positions
            .iter()
            .map(|&p| self.prover_matrix_index.row_poly.tree.prove(p))
            .collect::<Vec<_>>();
        let mut row_proofs = Vec::new();
        for row_proof in row_proofs_results {
            row_proofs.push(row_proof?);
        }
        let row_queried = OracleQueries::<B, E, H>::new(row_queried_evaluations, row_proofs);

        let col_queried_evaluations = queried_positions
            .iter()
            .map(|&p| E::from(self.prover_matrix_index.col_poly.evaluations[p]))
            .collect::<Vec<_>>();
        let col_proofs_results = queried_positions
            .iter()
            .map(|&p| self.prover_matrix_index.col_poly.tree.prove(p))
            .collect::<Vec<_>>();
        let mut col_proofs = Vec::new();
        for col_proof in col_proofs_results {
            col_proofs.push(col_proof?);
        }
        let col_queried = OracleQueries::<B, E, H>::new(col_queried_evaluations, col_proofs);

        let val_queried_evaluations = queried_positions
            .iter()
            .map(|&p| E::from(self.prover_matrix_index.val_poly.evaluations[p]))
            .collect::<Vec<_>>();
        let val_proofs_results = queried_positions
            .iter()
            .map(|&p| self.prover_matrix_index.val_poly.tree.prove(p))
            .collect::<Vec<_>>();
        let mut val_proofs = Vec::new();
        for val_proof in val_proofs_results {
            val_proofs.push(val_proof?);
        }
        let val_queried = OracleQueries::<B, E, H>::new(val_queried_evaluations, val_proofs);

        let t_alpha_transposed_evaluations = transpose_slice::<_, { n }>(&t_alpha_evals.clone());
        let hashed_evaluations = hash_values::<H, B, { n }>(&t_alpha_transposed_evaluations);
        let t_alpha_tree = MerkleTree::<H>::new(hashed_evaluations)?;
        let t_alpha_commitment = *t_alpha_tree.root();
        let t_alpha_queried_evaluations = queried_positions
            .iter()
            .map(|&p| E::from(t_alpha_evals[p]))
            .collect::<Vec<_>>();
        let t_alpha_proofs_results = queried_positions
            .iter()
            .map(|&p| t_alpha_tree.prove(p))
            .collect::<Vec<_>>();
        let mut t_alpha_proofs = Vec::new();
        for t_alpha_proof in t_alpha_proofs_results {
            t_alpha_proofs.push(t_alpha_proof?);
        }
        let t_alpha_queried =
            OracleQueries::<B, E, H>::new(t_alpha_queried_evaluations, t_alpha_proofs);
        Ok(LincheckProof::<B, E, H> {
            options: self.options.fri_options.clone(),
            num_evaluations: self.options.evaluation_domain.len(),
            alpha: self.alpha,
            beta,
            t_alpha_commitment,
            t_alpha_queried,
            products_sumcheck_proof,
            gamma,
            row_queried,
            col_queried,
            val_queried,
            matrix_sumcheck_proof,
            _e: PhantomData,
        })
        // unimplemented!()
    }
}
