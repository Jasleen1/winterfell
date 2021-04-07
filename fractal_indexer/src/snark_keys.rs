use common::proof::StarkProof;
use crypto::{MerkleTree, HashFunction};
use math::{field::{BaseElement, FieldElement, StarkField}};
use crate::{r1cs::*, index::*, indexed_matrix::*};
#[derive(Clone, Debug)]
pub struct ProverIndexPolynomial<E: FieldElement> {
    polynomial: Vec<E>,
    evaluations: Vec<E>, 
    tree: MerkleTree,
}

#[derive(Clone, Debug)]
pub struct ProverMatrixIndex<E: FieldElement> {
    matrix: Matrix<E>,
    row_poly: ProverIndexPolynomial<E>,
    col_poly: ProverIndexPolynomial<E>,
    val_poly: ProverIndexPolynomial<E>,
}

#[derive(Clone, Debug)]
pub struct ProverKey<E: FieldElement> {
    params: IndexParams,
    matrix_a_index: ProverMatrixIndex<E>,
    matrix_b_index: ProverMatrixIndex<E>,
    matrix_c_index: ProverMatrixIndex<E>,
}

#[derive(Clone, Debug)]
pub struct VerifierMatrixIndex { 
    row_poly_commitment: [u8; 32],
    col_poly_commitment: [u8; 32],
    val_poly_commitment: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct VerifierKey {
    params: IndexParams,
    matrix_a_commitments: VerifierMatrixIndex,
    matrix_b_commitments: VerifierMatrixIndex,
    matrix_c_commitments: VerifierMatrixIndex,
}

// QUESTION: Currently using the utils hash_values function which uses quartic folding.
// Is there any drawback to doing this here, where there's no layering?
pub fn commit_polynomial_evaluations<E: StarkField>(evaluations: &Vec<E>, hash_fn: HashFunction) -> MerkleTree {
    let transposed_evaluations = fri::folding::quartic::transpose(evaluations, 1);
    let hashed_evaluations = fri::folding::quartic::hash_values(&transposed_evaluations, hash_fn);
    MerkleTree::new(hashed_evaluations, hash_fn)
}

pub fn generate_prover_and_verifier_matrix_index<E: StarkField>(indexed: IndexedMatrix<E>, hash_fn: HashFunction) -> (ProverMatrixIndex<E>, VerifierMatrixIndex){
    let matrix = indexed.matrix;
    let row_polynomial = indexed.row_poly;
    let col_polynomial = indexed.col_poly;
    let val_polynomial = indexed.val_poly;
    let row_evals = indexed.row_evals_on_l;
    let col_evals = indexed.col_evals_on_l;
    let val_evals = indexed.val_evals_on_l;
    let row_tree = commit_polynomial_evaluations(&row_evals, hash_fn);
    let col_tree = commit_polynomial_evaluations(&col_evals, hash_fn);
    let val_tree = commit_polynomial_evaluations(&val_evals, hash_fn);
    let row_poly_commitment = *row_tree.root();
    let col_poly_commitment = *col_tree.root();
    let val_poly_commitment = *val_tree.root();

    let row_poly = ProverIndexPolynomial {polynomial: row_polynomial, evaluations: row_evals, tree: row_tree};
    let col_poly = ProverIndexPolynomial {polynomial: col_polynomial, evaluations: col_evals, tree: col_tree};
    let val_poly = ProverIndexPolynomial {polynomial: val_polynomial, evaluations: val_evals, tree: val_tree};
    let prover_matrix_index = ProverMatrixIndex { matrix, row_poly, col_poly, val_poly };
    let verifier_matrix_index = VerifierMatrixIndex {row_poly_commitment, col_poly_commitment, val_poly_commitment};
    (prover_matrix_index, verifier_matrix_index)
}

pub fn generate_prover_and_verifier_keys<E: StarkField>(Index { params, indexed_a, indexed_b, indexed_c }: Index<E>, hash_fn: HashFunction) -> (ProverKey<E>, VerifierKey) {
    let (matrix_a_index, matrix_a_commitments) = generate_prover_and_verifier_matrix_index(indexed_a, hash_fn);
    let (matrix_b_index, matrix_b_commitments) = generate_prover_and_verifier_matrix_index(indexed_b, hash_fn);
    let (matrix_c_index, matrix_c_commitments) = generate_prover_and_verifier_matrix_index(indexed_c, hash_fn);
    (ProverKey {params: params.clone(), matrix_a_index, matrix_b_index, matrix_c_index}, VerifierKey {params, matrix_a_commitments, matrix_b_commitments, matrix_c_commitments})
}

pub fn generate_basefield_keys(params: IndexParams, r1cs_instance: R1CS<BaseElement>, hash_fn: HashFunction) -> (ProverKey<BaseElement>, VerifierKey) {
    let index = create_basefield_index_from_r1cs(params, r1cs_instance);
    generate_prover_and_verifier_keys(index, hash_fn)
}