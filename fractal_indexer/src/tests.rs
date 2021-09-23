use std::vec;

use crate::{errors::R1CSError, index::*, r1cs::*, *};
use indexed_matrix::IndexedMatrix;
use math::{FieldElement, StarkField, fields::f128::{self, BaseElement}};

type SmallFieldElement17 = math::fields::smallprimefield::BaseElement<17, 3>;

#[test]
fn test_construct_matrix_f128() {
    let m: Result<Matrix<f128::BaseElement>, R1CSError> = make_all_ones_matrix_f128("dummy", 1, 1);
    let matrix = m.unwrap();

    let (r, c) = matrix.dims;
    assert!(r == 1);
    assert!(c == 1);
    for i in 0..1 {
        for j in 0..1 {
            assert!(matrix.mat[i][j] == f128::BaseElement::ONE);
        }
    }
    assert!(matrix.name == "dummy");
}

#[test]
fn test_construct_matrix_f17() {
    let m: Result<Matrix<SmallFieldElement17>, R1CSError> = make_all_ones_matrix_f17("dummy", 1, 1);
    let matrix = m.unwrap();

    let (r, c) = matrix.dims;
    assert!(r == 1);
    assert!(c == 1);
    for i in 0..1 {
        for j in 0..1 {
            assert!(matrix.mat[i][j] == SmallFieldElement17::ONE);
        }
    }
    assert!(matrix.name == "dummy");
}
/// This test should pass
#[test]
fn test_construct_r1cs() {
    let m1 = make_all_ones_matrix_f128("A", 1, 1);
    let matrix_a = m1.unwrap();
    let m2 = make_all_ones_matrix_f128("B", 1, 1);
    let matrix_b = m2.unwrap();
    let m3 = make_all_ones_matrix_f128("C", 1, 1);
    let matrix_c = m3.unwrap();

    let r1cs_instance = R1CS::new(matrix_a, matrix_b, matrix_c);
    assert!(r1cs_instance.is_ok());
}

#[test]
fn test_indexing() {
    let m1 = make_all_ones_matrix_f128("A", 2, 2);
    let matrix_a = m1.unwrap();
    let m2 = make_all_ones_matrix_f128("B", 2, 2);
    let matrix_b = m2.unwrap();
    let m3 = make_all_ones_matrix_f128("C", 2, 2);
    let matrix_c = m3.unwrap();

    // QUESTION: For all uses of clone(), below, is there a better way to pass the value? Perhaps as
    // an immutable reference?
    let r1cs_instance_result = R1CS::new(matrix_a, matrix_b, matrix_c);
    let r1cs_instance = r1cs_instance_result.unwrap();
    let params = IndexParams {
        num_input_variables: 2,
        num_constraints: 2,
        num_non_zero: 4,
    };
    let domains = build_index_domains(params.clone());
    let indexed_a = IndexedMatrix::new(r1cs_instance.A, domains.clone());
    let indexed_b = IndexedMatrix::new(r1cs_instance.B, domains.clone());
    let indexed_c = IndexedMatrix::new(r1cs_instance.C, domains);
    let index = Index::new(params, indexed_a, indexed_b, indexed_c);
    println!("Index is {:?}", index);
}

#[test]
fn test_domain_building_17() {
    let params = IndexParams {
        num_input_variables: 2,
        num_constraints: 2,
        num_non_zero: 4,
    };
    let domains = build_primefield_index_domains(params.clone());
    let h_field_base = domains.h_field_base;
    let i_field_base = domains.i_field_base;
    let k_field_base = domains.k_field_base;
    let l_field_base = domains.l_field_base;
    assert_eq!(h_field_base, SmallFieldElement17::new(16));
    assert_eq!(i_field_base, SmallFieldElement17::new(16));
    assert_eq!(k_field_base, SmallFieldElement17::new(13));
    assert_eq!(l_field_base, SmallFieldElement17::new(3));
}

#[test]
fn test_getting_roots_17() {
    let test_root_16 = SmallFieldElement17::get_root_of_unity(16);
    assert_eq!(test_root_16, SmallFieldElement17::new(3));
    let test_root_8 = SmallFieldElement17::get_root_of_unity(8);
    assert_eq!(test_root_8, SmallFieldElement17::new(9));
    let test_root_2 = SmallFieldElement17::get_root_of_unity(2);
    assert_eq!(test_root_2, SmallFieldElement17::new(16));
}

#[test]
fn test_single_indexed_matrix_17() {
    let m1 = make_all_ones_matrix_f17("A", 2, 2);
    let matrix_a = m1.unwrap();
    let params = IndexParams {
        num_input_variables: 2,
        num_constraints: 2,
        num_non_zero: 4,
    };
    let domains = build_index_domains(params.clone());
    println!("Domains {:?}", domains);
    let indexed_a = IndexedMatrix::new(matrix_a, domains.clone());
    println!("Indexed a is {:?}", indexed_a);
    let row_poly = indexed_a.row_poly;
    let col_poly = indexed_a.col_poly;
    let expected_row_poly = vec![0, 0, 1, 0];
    let expected_col_poly = vec![0, 11, 0, 7];
    for i in 0..4 {
        assert_eq!(row_poly[i], SmallFieldElement17::new(expected_row_poly[i]));
        assert_eq!(col_poly[i], SmallFieldElement17::new(expected_col_poly[i]));
    }
}

#[test]
fn test_indexing_f17() {
    let m1 = make_all_ones_matrix_f17("A", 2, 2);
    let matrix_a = m1.unwrap();
    let m2 = make_all_ones_matrix_f17("B", 2, 2);
    let matrix_b = m2.unwrap();
    let m3 = make_all_ones_matrix_f17("C", 2, 2);
    let matrix_c = m3.unwrap();

    // QUESTION: For all uses of clone(), below, is there a better way to pass the value? Perhaps as
    // an immutable reference?
    let r1cs_instance_result = R1CS::new(matrix_a, matrix_b, matrix_c);
    let r1cs_instance = r1cs_instance_result.unwrap();
    let params = IndexParams {
        num_input_variables: 2,
        num_constraints: 2,
        num_non_zero: 4,
    };
    let domains = build_primefield_index_domains(params.clone());
    let indexed_a = IndexedMatrix::new(r1cs_instance.A, domains.clone());
    let indexed_b = IndexedMatrix::new(r1cs_instance.B, domains.clone());
    let indexed_c = IndexedMatrix::new(r1cs_instance.C, domains);
    let index = Index::new(params, indexed_a, indexed_b, indexed_c);
    println!("Index is {:?}", index);
}

/// ***************  HELPERS *************** \\\
fn make_all_ones_matrix_f128(
    matrix_name: &str,
    rows: usize,
    cols: usize,
) -> Result<Matrix<BaseElement>, R1CSError> {
    let mut mat = Vec::new();
    let ones_row = vec![BaseElement::ONE; cols];
    for _i in 0..rows {
        mat.push(ones_row.clone());
    }
    Matrix::new(matrix_name, mat)
}

fn make_all_ones_matrix_f17(
    matrix_name: &str,
    rows: usize,
    cols: usize,
) -> Result<Matrix<SmallFieldElement17>, R1CSError> {
    let mut mat = Vec::new();
    let ones_row = vec![SmallFieldElement17::ONE; cols];
    for _i in 0..rows {
        mat.push(ones_row.clone());
    }
    Matrix::new(matrix_name, mat)
}
