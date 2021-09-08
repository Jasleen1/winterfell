use std::convert::TryInto;

use fractal_indexer::{indexed_matrix::*, snark_keys::*};
use fractal_utils::{errors::MatrixError, matrix_utils::*, polynomial_utils::*, *};
use fri::{FriOptions, FriProof, DefaultProverChannel, PublicCoin};
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
    utils,
};

use fractal_proofs::{SumcheckProof, LincheckProof, MatrixArithProof};

// TODO: Will need to ask Irakliy whether a channel should be passed in here
pub struct LincheckProver<E: FieldElement + From<BaseElement>> {
    alpha: E,
    beta: E, 
    prover_matrix_index: ProverMatrixIndex<E>,
    f_1_poly_coeffs: Vec<E>,
    f_2_poly_coeffs: Vec<E>,
    degree_fs: usize,
    size_subgroup_h: u128,
    size_subgroup_k: u128,
    summing_domain: Vec<BaseElement>,
    evaluation_domain: Vec<BaseElement>,
    fri_options: FriOptions,
    num_queries: usize,
}

impl<E: FieldElement + From<BaseElement>> LincheckProver<E> {
    pub fn new(
        alpha: E,
        beta: E, 
        prover_matrix_index: ProverMatrixIndex<E>,
        f_1_poly_coeffs: Vec<E>,
        f_2_poly_coeffs: Vec<E>,
        degree_fs: usize,
        size_subgroup_h: u128,
        size_subgroup_k: u128,
        summing_domain: Vec<BaseElement>,
        evaluation_domain: Vec<BaseElement>,
        fri_options: FriOptions,
        num_queries: usize,
    ) -> Self {
        LincheckProver{
            alpha,
            beta, 
            prover_matrix_index,
            f_1_poly_coeffs,
            f_2_poly_coeffs,
            degree_fs,
            size_subgroup_h,
            size_subgroup_k,
            summing_domain,
            evaluation_domain,
            fri_options,
            num_queries,
        }
    }
    
    pub fn generate_t_alpha(&self) -> Vec<E> {
        let v_h_alpha = vanishing_poly_for_mult_subgroup(self.alpha, self.size_subgroup_h);
        let mut coefficient_values = Vec::new();
        for id in 0..self.summing_domain.len() {
            let summing_elt = E::from(self.summing_domain[id]);
            let denom_term = self.alpha - self.prover_matrix_index.get_col_eval(summing_elt);
            let k_term_factor = denom_term.inv();
            let k_term = self.prover_matrix_index.get_val_eval(summing_elt) * k_term_factor;
            coefficient_values.push(k_term)
        }
        let mut t_evals = Vec::new();
        for x_val_id in 0..self.evaluation_domain.len() {
            let x_val = E::from(self.evaluation_domain[x_val_id]);
            let v_h_x = vanishing_poly_for_mult_subgroup(x_val, self.size_subgroup_h);
            
            let mut sum_without_vs = E::ZERO;
            for id in 0..self.summing_domain.len() {
                let summing_elt = E::from(self.summing_domain[id]);
                let denom_term: E = x_val - self.prover_matrix_index.get_row_eval(summing_elt);
                let prod_term = coefficient_values[id] * denom_term.inv();
                sum_without_vs = sum_without_vs + prod_term;
            }  
            let sum_with_vs = (sum_without_vs * v_h_x) * v_h_alpha;
            t_evals.push(sum_with_vs);
        } 
        t_evals

    }


    pub fn generate_lincheck_proof(&self) -> LincheckProof<E> {
        // Compute t(X, alpha) 
        unimplemented!()

    } 
}