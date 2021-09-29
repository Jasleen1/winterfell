use std::marker::PhantomData;

use crypto::ElementHasher;
use fractal_indexer::snark_keys::*;
use fractal_utils::polynomial_utils::*;
use fri::FriOptions;
use math::{FieldElement, StarkField};

use fractal_proofs::LincheckProof;

// TODO: Will need to ask Irakliy whether a channel should be passed in here
pub struct LincheckProver<
    B: StarkField,
    E: FieldElement<BaseField = B>,
    H: ElementHasher + ElementHasher<BaseField = B>,
> {
    alpha: B,
    beta: B,
    prover_matrix_index: ProverMatrixIndex<H, B>,
    f_1_poly_coeffs: Vec<E>,
    f_2_poly_coeffs: Vec<E>,
    degree_fs: usize,
    size_subgroup_h: u128,
    size_subgroup_k: u128,
    summing_domain: Vec<B>,
    evaluation_domain: Vec<B>,
    fri_options: FriOptions,
    num_queries: usize,
    _h: PhantomData<H>,
}

impl<
        B: StarkField,
        E: FieldElement<BaseField = B>,
        H: ElementHasher + ElementHasher<BaseField = B>,
    > LincheckProver<B, E, H>
{
    pub fn new(
        alpha: B,
        beta: B,
        prover_matrix_index: ProverMatrixIndex<H, B>,
        f_1_poly_coeffs: Vec<E>,
        f_2_poly_coeffs: Vec<E>,
        degree_fs: usize,
        size_subgroup_h: u128,
        size_subgroup_k: u128,
        summing_domain: Vec<B>,
        evaluation_domain: Vec<B>,
        fri_options: FriOptions,
        num_queries: usize,
    ) -> Self {
        LincheckProver {
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
            _h: PhantomData,
        }
    }

    pub fn generate_t_alpha(&self) -> Vec<B> {
        let v_h_alpha = vanishing_poly_for_mult_subgroup(self.alpha, self.size_subgroup_h);
        let mut coefficient_values = Vec::new();
        for id in 0..self.summing_domain.len() {
            let summing_elt = self.summing_domain[id];
            let denom_term = self.alpha - self.prover_matrix_index.get_col_eval(summing_elt);
            let k_term_factor = denom_term.inv();
            let k_term = self.prover_matrix_index.get_val_eval(summing_elt) * k_term_factor;
            coefficient_values.push(k_term)
        }
        let mut t_evals = Vec::new();
        for x_val_id in 0..self.evaluation_domain.len() {
            let x_val = self.evaluation_domain[x_val_id];
            let v_h_x = vanishing_poly_for_mult_subgroup(x_val, self.size_subgroup_h);

            let mut sum_without_vs = B::ZERO;
            for id in 0..self.summing_domain.len() {
                let summing_elt = self.summing_domain[id];
                let denom_term = x_val - self.prover_matrix_index.get_row_eval(summing_elt);
                let prod_term = coefficient_values[id] * denom_term.inv();
                sum_without_vs = sum_without_vs + prod_term;
            }
            let sum_with_vs = (sum_without_vs * v_h_x) * v_h_alpha;
            t_evals.push(sum_with_vs);
        }
        t_evals
    }

    pub fn generate_lincheck_proof(&self) -> LincheckProof<B, E, H> {
        // Compute t(X, alpha)
        unimplemented!()
    }
}
