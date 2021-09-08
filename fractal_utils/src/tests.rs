use crate::{errors::MatrixError, matrix_utils::*};
use math::{
    fft,
    field::{BaseElement, FieldElement, SmallFieldElement13, SmallFieldElement17, StarkField},
    utils,
};
use std::vec;

#[test]
fn test_matrix_star() {
    let original_matrix = make_all_ones_matrix_f17("test", 2, 2).unwrap();
    let h_field_base = SmallFieldElement17::get_root_of_unity(2);
    let h_field = SmallFieldElement17::get_power_series(h_field_base, 2);
    let matrix_star = original_matrix.get_matrix_star(h_field.clone()).unwrap();
    let expected = vec![vec![2, 15], vec![2, 15]];
    for i in 0..2 {
        for j in 0..2 {
            assert_eq!(
                matrix_star.get_value(i, j),
                SmallFieldElement17::new(expected[i][j])
            );
        }
    }
}

fn make_all_ones_matrix_f17(
    matrix_name: &str,
    rows: usize,
    cols: usize,
) -> Result<Matrix<SmallFieldElement17>, MatrixError> {
    let mut mat = Vec::new();
    let ones_row = vec![SmallFieldElement17::ONE; cols];
    for _i in 0..rows {
        mat.push(ones_row.clone());
    }
    Matrix::new(matrix_name, mat)
}
