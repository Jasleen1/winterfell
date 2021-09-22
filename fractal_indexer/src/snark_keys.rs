use crate::{index::{create_basefield_index_from_r1cs, Index, IndexParams}, indexed_matrix::IndexedMatrix, r1cs::{Matrix, R1CS}, errors::*};
use crypto::{ElementHasher, Hasher, MerkleTree};
use utils::transpose_slice;
use math::{FieldElement, StarkField, fields::f128::BaseElement};
use fri::utils::hash_values;
#[derive(Debug)]
pub struct ProverIndexPolynomial<H: Hasher, E: FieldElement> {
    polynomial: Vec<E>,
    evaluations: Vec<E>,
    tree: MerkleTree<H>,
}

impl<H: Hasher, E: FieldElement> ProverIndexPolynomial<H, E> {
    // TODO Add error checking, currently assumes index is
    // within range.
    pub fn get_eval_at_index(&self, index: usize) -> E {
        self.evaluations[index]
    }

    pub fn get_eval_at_point(&self, point: E) -> E {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct ProverMatrixIndex<H: Hasher, E: FieldElement> {
    pub matrix: Matrix<E>,
    pub row_poly: ProverIndexPolynomial<H, E>,
    pub col_poly: ProverIndexPolynomial<H, E>,
    pub val_poly: ProverIndexPolynomial<H, E>,
}

impl<H: Hasher, E: FieldElement> ProverMatrixIndex<H, E> {
    pub fn get_val_eval(&self, point: E) -> E {
        self.val_poly.get_eval_at_point(point)
    }
    pub fn get_val_eval_at_index(&self, index: usize) -> E {
        self.val_poly.get_eval_at_index(index)
    }

    pub fn get_col_eval(&self, point: E) -> E {
        self.col_poly.get_eval_at_point(point)
    }
    pub fn get_col_eval_at_index(&self, index: usize) -> E {
        self.col_poly.get_eval_at_index(index)
    }

    pub fn get_row_eval(&self, point: E) -> E {
        self.row_poly.get_eval_at_point(point)
    }
    pub fn get_row_eval_at_index(&self, index: usize) -> E {
        self.row_poly.get_eval_at_index(index)
    }

}

#[derive(Debug)]
pub struct ProverKey<H: Hasher, E: FieldElement> {
    params: IndexParams,
    matrix_a_index: ProverMatrixIndex<H, E>,
    matrix_b_index: ProverMatrixIndex<H, E>,
    matrix_c_index: ProverMatrixIndex<H, E>,
}

#[derive(Debug)]
pub struct VerifierMatrixIndex<H: Hasher> {
    row_poly_commitment: H::Digest,
    col_poly_commitment: H::Digest,
    val_poly_commitment: H::Digest,
}

#[derive(Debug)]
pub struct VerifierKey<H: Hasher> {
    params: IndexParams,
    matrix_a_commitments: VerifierMatrixIndex<H>,
    matrix_b_commitments: VerifierMatrixIndex<H>,
    matrix_c_commitments: VerifierMatrixIndex<H>,
}

// QUESTION: Currently using the utils hash_values function which uses quartic folding.
// Is there any drawback to doing this here, where there's no layering?
pub fn commit_polynomial_evaluations<H: ElementHasher, E: StarkField, const N: usize>(
    evaluations: &Vec<E>
) -> Result<MerkleTree<H>, IndexerError> {
    let transposed_evaluations = transpose_slice(evaluations);
    let hashed_evaluations = hash_values::<H, E, N>(&transposed_evaluations);
    Ok(MerkleTree::<H>::new(hashed_evaluations)?)
}

pub fn generate_prover_and_verifier_matrix_index<H: ElementHasher, E: StarkField>(
    indexed: IndexedMatrix<E>
) -> Result<(ProverMatrixIndex<H, E>, VerifierMatrixIndex<H>), IndexerError> {
    let matrix = indexed.matrix;
    let row_polynomial = indexed.row_poly;
    let col_polynomial = indexed.col_poly;
    let val_polynomial = indexed.val_poly;
    let row_evals = indexed.row_evals_on_l;
    let col_evals = indexed.col_evals_on_l;
    let val_evals = indexed.val_evals_on_l;
    let row_tree = commit_polynomial_evaluations(&row_evals)?;
    let col_tree = commit_polynomial_evaluations(&col_evals)?;
    let val_tree = commit_polynomial_evaluations(&val_evals)?;
    let row_poly_commitment = *row_tree.root();
    let col_poly_commitment = *col_tree.root();
    let val_poly_commitment = *val_tree.root();

    let row_poly = ProverIndexPolynomial {
        polynomial: row_polynomial,
        evaluations: row_evals,
        tree: row_tree,
    };
    let col_poly = ProverIndexPolynomial {
        polynomial: col_polynomial,
        evaluations: col_evals,
        tree: col_tree,
    };
    let val_poly = ProverIndexPolynomial {
        polynomial: val_polynomial,
        evaluations: val_evals,
        tree: val_tree,
    };
    let prover_matrix_index = ProverMatrixIndex {
        matrix,
        row_poly,
        col_poly,
        val_poly,
    };
    let verifier_matrix_index = VerifierMatrixIndex {
        row_poly_commitment,
        col_poly_commitment,
        val_poly_commitment,
    };
    Ok((prover_matrix_index, verifier_matrix_index))
}

pub fn generate_prover_and_verifier_keys<E: StarkField, H: Hasher>(
    Index {
        params,
        indexed_a,
        indexed_b,
        indexed_c,
    }: Index<E>,
) -> Result<(ProverKey<H, E>, VerifierKey<H>), IndexerError> {
    let (matrix_a_index, matrix_a_commitments) =
        generate_prover_and_verifier_matrix_index(indexed_a)?;
    let (matrix_b_index, matrix_b_commitments) =
        generate_prover_and_verifier_matrix_index(indexed_b)?;
    let (matrix_c_index, matrix_c_commitments) =
        generate_prover_and_verifier_matrix_index(indexed_c)?;
    Ok((
        ProverKey {
            params: params.clone(),
            matrix_a_index,
            matrix_b_index,
            matrix_c_index,
        },
        VerifierKey {
            params,
            matrix_a_commitments,
            matrix_b_commitments,
            matrix_c_commitments,
        },
    ))
}

pub fn generate_basefield_keys<H: Hasher>(
    params: IndexParams,
    r1cs_instance: R1CS<BaseElement>,
) -> (ProverKey<H, BaseElement>, VerifierKey<H>) {
    let index = create_basefield_index_from_r1cs(params, r1cs_instance);
    generate_prover_and_verifier_keys::<BaseElement, H>(index)
}
