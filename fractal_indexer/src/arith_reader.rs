use math::StarkField;

use crate::{errors::ParserError, r1cs::*};

#[test]
fn test_parser() {

}

pub fn read_and_parse_arith_file<E: StarkField>(filename: &str) -> Result<R1CS<E>, ParserError> {
    unimplemented!()
}

pub fn handle_addition<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    unimplemented!()
}

pub fn handle_const_addition<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    unimplemented!()
}

pub fn handle_var_mult<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    // SIMPLE MULT ******
    // mul in 2 <43 44> out 1 <45>
    // Means matrix_A position 43 should be 1, 
    // matrix_B position 44 should be 1, matrix_C position 45 should be 1

    // CONSTANT & SIMPLE MULT *****
    // Multiply one variable by a constant followed by multiplying two variables together.
    // const-mul-14 in 1 <45> out 1 <48>
    // This is (0x14*<x_45>) *  (1 * <0>) = <x_48>
    // matrix_A position 45 = Ox14
    // matrix_B position 0 = 1
    // matrix_C position 48 = 1 
    // A = [0... 0x14...0] 
    // B = [1 0...]
    // C = [0.... 1 ...0] (1 in position 48)
    // Ax * Bx = Cx
    // INPUT VEC = [1, x_1, ...., x_45, ....]
    // mul in 2 <47 48> out 1 <49>
    // matrix_A position 47 = 1
    // matrix_B position 48 = 1
    // matrix_C position 49 = 1

    // OPTIMIZATION ******
    // const-mul-14 in 1 <45> out 1 <48>
    // mul in 2 <47 48> out 1 <49>
    // matrix_A position 47 = 1
    // matrix_B position 45 = 0x14
    // matrix_C position 49 = 1


    unimplemented!()
}

pub fn handle_const_mult<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    unimplemented!()
}

pub fn handle_xor_constraint<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    unimplemented!()
}

pub fn handle_or_constraint<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    unimplemented!()
}

pub fn handle_assert_constraint<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    unimplemented!()
}

pub fn handle_const_mul_constraint<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    unimplemented!()
}

pub fn handle_neg_const_mul_constraint<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    unimplemented!()
}

pub fn handle_non_zero_check_constraint<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    unimplemented!()
}

pub fn handle_split_constraint<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    unimplemented!()
}

pub fn handle_pack_constraint<E: StarkField>(input_str: &str, output_str: &str, matrix: &mut R1CS<E>) {
    unimplemented!()
}

