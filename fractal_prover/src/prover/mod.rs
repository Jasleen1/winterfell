use std::marker::PhantomData;

use crypto::{ElementHasher, RandomCoin};
use fractal_indexer::{r1cs::Matrix, snark_keys::*};

use math::{FieldElement, StarkField};

use fractal_proofs::{fft, FractalProof, TryInto};

use crate::{
    errors::ProverError, lincheck_prover::LincheckProver, rowcheck_prover::RowcheckProver,
    FractalOptions,
};

pub struct FractalProver<
    B: StarkField,
    E: FieldElement<BaseField = B>,
    H: ElementHasher + ElementHasher<BaseField = B>,
> {
    prover_key: ProverKey<H, B>,
    options: FractalOptions<B>,
    witness: Vec<B>,
    variable_assignment: Vec<B>,
    public_coin: RandomCoin<B, H>,
    _e: PhantomData<E>,
}

impl<
        B: StarkField,
        E: FieldElement<BaseField = B>,
        H: ElementHasher + ElementHasher<BaseField = B> + Clone,
    > FractalProver<B, E, H>
{
    pub fn new(
        prover_key: ProverKey<H, B>,
        options: FractalOptions<B>,
        witness: Vec<B>,
        variable_assignment: Vec<B>,
        pub_inputs_bytes: Vec<u8>,
    ) -> Self {
        let coin_seed = pub_inputs_bytes;
        FractalProver {
            prover_key,
            options,
            witness,
            variable_assignment,
            public_coin: RandomCoin::new(&coin_seed),
            _e: PhantomData,
        }
    }

    pub fn generate_proof(&mut self) -> Result<FractalProof<B, E, H>, ProverError> {
        // This is the less efficient version and assumes only dealing with the var assignment,
        // not z = (x, w)
        let alpha = self.public_coin.draw().expect("failed to draw OOD point");
        let inv_twiddles_h = fft::get_inv_twiddles(self.variable_assignment.len());
        let f_1_a_poly = &mut self.variable_assignment.clone();
        fft::interpolate_poly(f_1_a_poly, &inv_twiddles_h);
        let lincheck_prover_a = LincheckProver::<B, E, H>::new(
            alpha,
            self.prover_key.matrix_a_index.clone(),
            f_1_a_poly.to_vec(),
            self.compute_matrix_mul_poly_coeffs("a", &inv_twiddles_h)?,
            self.options.clone(),
        );
        let lincheck_a = lincheck_prover_a.generate_lincheck_proof()?;
        let f_1_b_poly = &mut self.variable_assignment.clone();
        fft::interpolate_poly(f_1_b_poly, &inv_twiddles_h);
        let lincheck_prover_b = LincheckProver::<B, E, H>::new(
            alpha,
            self.prover_key.matrix_b_index.clone(),
            f_1_b_poly.to_vec(),
            self.compute_matrix_mul_poly_coeffs("b", &inv_twiddles_h)?,
            self.options.clone(),
        );
        let lincheck_b = lincheck_prover_b.generate_lincheck_proof()?;
        let f_1_c_poly = &mut self.variable_assignment.clone();
        fft::interpolate_poly(f_1_c_poly, &inv_twiddles_h);
        let lincheck_prover_c = LincheckProver::<B, E, H>::new(
            alpha,
            self.prover_key.matrix_c_index.clone(),
            f_1_c_poly.to_vec(),
            self.compute_matrix_mul_poly_coeffs("c", &inv_twiddles_h)?,
            self.options.clone(),
        );
        let lincheck_c = lincheck_prover_c.generate_lincheck_proof()?;
        let eval_twiddles = fft::get_twiddles(self.options.evaluation_domain.len());
        let mut f_az_evals = f_1_a_poly.clone();
        fft::evaluate_poly(&mut f_az_evals, &eval_twiddles);
        let mut f_bz_evals = f_1_b_poly.clone();
        fft::evaluate_poly(&mut f_bz_evals, &eval_twiddles);
        let mut f_cz_evals = f_1_c_poly.clone();
        fft::evaluate_poly(&mut f_cz_evals, &eval_twiddles);
        let rowcheck_prover = RowcheckProver::<B, E, H>::new(
            f_az_evals,
            f_bz_evals,
            f_cz_evals,
            self.options.degree_fs,
            self.options.size_subgroup_h.try_into().unwrap(),
            self.options.evaluation_domain.clone(),
            self.options.fri_options.clone(),
            self.options.num_queries,
        );
        let rowcheck_proof = rowcheck_prover.generate_proof()?;
        Ok(FractalProof {
            rowcheck_proof,
            lincheck_a,
            lincheck_b,
            lincheck_c,
        })
        // unimplemented!()
    }

    fn compute_matrix_mul_poly_coeffs(
        &self,
        matrix_label: &str,
        inv_twiddles: &[B],
    ) -> Result<Vec<B>, ProverError> {
        let mut matrix = Matrix::new(matrix_label, Vec::<Vec<B>>::new())?;
        match matrix_label {
            "a" => {
                matrix = self.prover_key.matrix_a_index.matrix.clone();
            }
            "b" => {
                matrix = self.prover_key.matrix_b_index.matrix.clone();
            }
            "c" => {
                matrix = self.prover_key.matrix_c_index.matrix.clone();
            }
            _ => {}
        }
        if matrix.mat.len() == 0 {
            return Err(ProverError::InvalidMatrixName(matrix_label.to_string()));
        }
        let mut f_2_vals = matrix.dot(self.variable_assignment.clone());
        fft::interpolate_poly(&mut f_2_vals, inv_twiddles);
        Ok(f_2_vals)
    }
}
