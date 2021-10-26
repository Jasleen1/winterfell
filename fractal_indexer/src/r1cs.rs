use math::StarkField;

use crate::errors::*;

pub type MatrixDimensions = (usize, usize);
#[derive(Clone, Debug)]
pub struct Matrix<E: StarkField> {
    pub name: String,
    pub mat: Vec<Vec<E>>,
    pub dims: MatrixDimensions,
}

pub fn valid_matrix<E: StarkField>(
    name: &str,
    matrix: Vec<Vec<E>>,
) -> Result<Matrix<E>, R1CSError> {
    let rows = matrix.len();
    if rows == 0 {
        let dims = (0, 0);
        return Ok(Matrix {
            name: String::from(name),
            mat: matrix,
            dims: dims,
        });
    } else {
        let cols = matrix[0].len();
        for i in 0..rows {
            if matrix[i].len() != cols {
                return Err(R1CSError::InvalidMatrix(String::from(name)));
            }
        }
        let dims = (rows, cols);
        return Ok(Matrix {
            name: String::from(name),
            mat: matrix,
            dims,
        });
    }
}

impl<B: StarkField> Matrix<B> {
    pub fn new(name: &str, matrix: Vec<Vec<B>>) -> Result<Self, R1CSError> {
        let valid = valid_matrix(name, matrix);
        match valid {
            Ok(m) => Ok(m),
            Err(e) => Err(e),
        }
    }

    pub fn get_total_size(&self) -> usize {
        let rows = self.dims.0;
        let cols = self.dims.1;
        let total_size = rows + cols;
        return total_size;
    }

    pub fn dot(&self, vec: Vec<B>) -> Vec<B> {
        self.mat
            .iter()
            .map(|a| {
                a.iter()
                    .zip(vec.iter())
                    .map(|(x, y)| x.mul(*y))
                    .fold(B::ZERO, |sum, i| sum.add(i))
            })
            .collect()
    }
}

// TODO: Should A, B and C come with respective lengths
#[derive(Clone, Debug)]
pub struct R1CS<E: StarkField> {
    pub A: Matrix<E>,
    pub B: Matrix<E>,
    pub C: Matrix<E>,
}

// TODO Might want to change this to include checks for A, B and C.
impl<E: StarkField> R1CS<E> {
    pub fn new(
        matrix_a: Matrix<E>,
        matrix_b: Matrix<E>,
        matrix_c: Matrix<E>,
    ) -> Result<Self, R1CSError> {
        let valid = valid_r1cs(&matrix_a, &matrix_b, &matrix_c);
        match valid {
            Ok(_) => Ok(R1CS {
                A: matrix_a,
                B: matrix_b,
                C: matrix_c,
            }),
            Err(e) => Err(e),
        }
    }

    pub fn get_a(&mut self) -> &mut Matrix<E> {
        &mut self.A
    }

    pub fn get_b(&mut self) -> &mut Matrix<E> {
        &mut self.B
    }

    pub fn get_c(&mut self) -> &mut Matrix<E> {
        &mut self.C
    }
}

// TODO: indexed R1CS consisting of 3 indexed matrices

// TODO: Add error here

pub fn valid_r1cs<E: StarkField>(
    a: &Matrix<E>,
    b: &Matrix<E>,
    c: &Matrix<E>,
) -> Result<bool, crate::errors::R1CSError> {
    let a_dims = a.dims;
    let b_dims = b.dims;
    let c_dims = c.dims;
    if b_dims != a_dims {
        return Err(R1CSError::MatrixSizeMismatch(
            a.name.clone(),
            b.name.clone(),
        ));
    } else if c_dims != a_dims {
        return Err(R1CSError::MatrixSizeMismatch(
            a.name.clone(),
            c.name.clone(),
        ));
    } else {
        return Ok(true);
    }
}
