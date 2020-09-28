use common::utils::filled_vector;
use prover::math::{
    fft,
    field::{self, add, mul, sub},
};

// CONSTRAINT EVALUATION HELPERS
// ================================================================================================

pub fn are_equal(a: u128, b: u128) -> u128 {
    sub(a, b)
}

pub fn is_zero(a: u128) -> u128 {
    a
}

pub fn extend_cyclic_values(
    values: &[u128],
    inv_twiddles: &[u128],
    ev_twiddles: &[u128],
) -> (Vec<u128>, Vec<u128>) {
    let domain_size = ev_twiddles.len() * 2;
    let cycle_length = values.len();

    let mut extended_values = filled_vector(cycle_length, domain_size, field::ZERO);
    extended_values.copy_from_slice(values);
    fft::interpolate_poly(&mut extended_values, &inv_twiddles, true);

    let poly = extended_values.clone();

    unsafe {
        extended_values.set_len(extended_values.capacity());
    }
    fft::evaluate_poly(&mut extended_values, &ev_twiddles, true);

    (poly, extended_values)
}

// TRAIT TO SIMPLIFY CONSTRAINT AGGREGATION
// ================================================================================================

pub trait EvaluationResult {
    fn agg_constraint(&mut self, index: usize, flag: u128, value: u128);
}

impl EvaluationResult for [u128] {
    fn agg_constraint(&mut self, index: usize, flag: u128, value: u128) {
        self[index] = add(self[index], mul(flag, value));
    }
}

impl EvaluationResult for Vec<u128> {
    fn agg_constraint(&mut self, index: usize, flag: u128, value: u128) {
        self[index] = add(self[index], mul(flag, value));
    }
}

// OTHER FUNCTIONS
// ================================================================================================

/// Transposes columns into rows in a 2-dimensional matrix.
pub fn transpose(values: Vec<Vec<u128>>) -> Vec<Vec<u128>> {
    let mut result = Vec::new();

    let columns = values.len();
    assert!(columns > 0, "matrix must contain at least one column");

    let rows = values[0].len();
    assert!(rows > 0, "matrix must contain at least one row");

    for _ in 0..rows {
        result.push(vec![0; columns]);
    }

    for (i, column) in values.iter().enumerate() {
        assert!(
            column.len() == rows,
            "number of rows must be the same for all columns"
        );
        for j in 0..rows {
            result[j][i] = column[j];
        }
    }

    result
}

/// Prints out an execution trace
pub fn print_trace(trace: &[Vec<u128>]) {
    let trace_width = trace.len();
    let trace_length = trace[0].len();

    let mut state = vec![0; trace_width];
    for i in 0..trace_length {
        for j in 0..trace_width {
            state[j] = trace[j][i];
        }
        println!("{}\t{:?}", i, state);
    }
}

/// Converts a slice of u128 values into a vector of bytes.
pub fn to_byte_vec(values: &[u128]) -> Vec<u8> {
    let mut result = Vec::with_capacity(values.len() * 16);
    for value in values {
        result.extend_from_slice(&value.to_le_bytes());
    }
    result
}
