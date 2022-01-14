use core::num;

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

impl<E: StarkField> Matrix<E> {
    pub fn new(name: &str, matrix: Vec<Vec<E>>) -> Result<Self, R1CSError> {
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

    pub fn dot(&self, vec: Vec<E>) -> Vec<E> {
        self.mat
            .iter()
            .map(|a| {
                a.iter()
                    .zip(vec.iter())
                    .map(|(x, y)| x.mul(*y))
                    .fold(E::ZERO, |sum, i| sum.add(i))
            })
            .collect()
    }

    pub fn define_cols(&mut self, num_cols: usize) {
        self.dims.1 = num_cols;
    }

    pub fn add_row(&mut self, new_row: Vec<E>) {
        if new_row.len() != self.dims.1 {
           // FIXME: add error handling 
        }
        self.mat.push(new_row.clone());
        self.dims.0 = self.dims.0 + 1;
    }

    pub fn debug_print(&mut self) {
        println!("{}", self.name);
        for row in &self.mat {
            for elt in row {
                if elt == &E::ZERO {
                    print!("0 ");
                } else if elt == &E::ONE {
                    print!("1 ");
                } else {
                    print!("{:?}", elt);
                }
            }
            println!("");
        }
    }

    /// Print row as ...1..1.1...*...1.. with no newline.
    pub fn debug_print_row_bits(&mut self, row_idx: usize) {
        for elt in &self.mat[row_idx] {
            if elt == &E::ZERO {
                print!(".");
            } else if elt == &E::ONE {
                print!("1");
            } else {
                print!("*");
            }
        }
    }

    pub fn debug_print_bits(&mut self) {
        println!("{}", self.name);
        for row in &self.mat {
            for elt in row {
                if elt == &E::ZERO {
                    print!(".");
                } else if elt == &E::ONE {
                    print!("1");
                } else {
                    print!("*");
                }
            }
            println!("");
        }
    }
}

pub(crate) fn create_empty_matrix<E: StarkField>(name: String) -> Matrix<E> {
    Matrix {
        name, 
        mat: Vec::<Vec<E>>::new(),
        dims: (0, 0),
    }
}

pub(crate) fn create_empty_r1cs<E: StarkField>() -> Result<R1CS<E>, R1CSError> {
    let matrix_a = create_empty_matrix("A".to_string());
    let matrix_b = create_empty_matrix("B".to_string());
    let matrix_c = create_empty_matrix("C".to_string());
    R1CS::new(matrix_a, matrix_b, matrix_c)
}

// TODO: Should A, B and C come with respective lengths
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct R1CS<E: StarkField> {
    #[allow(non_snake_case)]
    pub A: Matrix<E>,
    #[allow(non_snake_case)]
    pub B: Matrix<E>,
    #[allow(non_snake_case)]
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

    pub fn get_num_cols(&self) -> usize {
        self.A.dims.1
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

    pub fn set_cols(&mut self, num_cols: usize) {
        self.A.define_cols(num_cols);
        self.B.define_cols(num_cols);
        self.C.define_cols(num_cols);
    }

    pub fn add_rows(&mut self, new_row_a: Vec<E>, new_row_b: Vec<E>, new_row_c: Vec<E>) {
        self.A.add_row(new_row_a);
        self.B.add_row(new_row_b);
        self.C.add_row(new_row_c);
    }

    pub fn debug_print(&mut self) {
        println!("Dimensions: {} {}", self.A.dims.0, self.A.dims.1);
        self.A.debug_print();
        self.B.debug_print();
        self.C.debug_print();
    }

    pub fn debug_print_bits(&mut self) {
        println!("Dimensions: {} {}", self.A.dims.0, self.A.dims.1);
        self.A.debug_print_bits();
        self.B.debug_print_bits();
        self.C.debug_print_bits();
    }

    pub fn debug_print_bits_horizontal(&mut self) {
        let num_rows = self.A.dims.0;
        for row_idx in 0..num_rows-1 {
            self.A.debug_print_row_bits(row_idx);
            print!("  ");
            self.B.debug_print_row_bits(row_idx);
            print!("  ");
            self.C.debug_print_row_bits(row_idx);
            println!("");
        }
    }

    fn debug_print_row_symbolic(&self, row: &Vec<E>) {
        let mut first = true;
        for col_idx in 0..row.len() {
            let elt = row[col_idx];
            if elt != E::ZERO {
                if first {
                    first = false;
                } else {
                    print!(" + ");
                }
                if col_idx == 0 {
                    print!("{}", elt);
                } else {
                    if elt == E::ONE {
                        print!("v{}", col_idx)
                    } else {
                        print!("{} v{}", elt, col_idx)
                    }
                }
            }
        }
    }

    pub fn debug_print_symbolic(&mut self) {
        let num_rows = self.A.dims.0;
        for row_idx in 0..num_rows-1 {
            print!("(");
            self.debug_print_row_symbolic(&self.A.mat[row_idx]);
            print!(")  (");
            self.debug_print_row_symbolic(&self.B.mat[row_idx]);
            print!(") == ");
            self.debug_print_row_symbolic(&self.C.mat[row_idx]);
            println!("");
        }
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
