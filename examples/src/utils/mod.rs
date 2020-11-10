use common::utils::filled_vector;
use prover::math::{
    fft,
    field::{FieldElement, FieldElementTrait, StarkField},
};

pub mod rescue;

// CONSTRAINT EVALUATION HELPERS
// ================================================================================================

pub fn are_equal(a: FieldElement, b: FieldElement) -> FieldElement {
    a - b
}

pub fn is_zero(a: FieldElement) -> FieldElement {
    a
}

pub fn is_binary(a: FieldElement) -> FieldElement {
    a * a - a
}

pub fn not(a: FieldElement) -> FieldElement {
    FieldElement::ONE - a
}

pub fn when(a: FieldElement, b: FieldElement) -> FieldElement {
    a * b
}

// TRAIT TO SIMPLIFY CONSTRAINT AGGREGATION
// ================================================================================================

pub trait EvaluationResult {
    fn agg_constraint(&mut self, index: usize, flag: FieldElement, value: FieldElement);
}

impl EvaluationResult for [FieldElement] {
    fn agg_constraint(&mut self, index: usize, flag: FieldElement, value: FieldElement) {
        self[index] = self[index] + flag * value;
    }
}

impl EvaluationResult for Vec<FieldElement> {
    fn agg_constraint(&mut self, index: usize, flag: FieldElement, value: FieldElement) {
        self[index] = self[index] + flag * value;
    }
}

// CYCLIC VALUES
// ================================================================================================

/// Builds extension domain for cyclic registers.
pub fn build_cyclic_domain(
    cycle_length: usize,
    blowup_factor: usize,
) -> (Vec<FieldElement>, Vec<FieldElement>) {
    let root = FieldElement::get_root_of_unity(cycle_length.trailing_zeros());
    let inv_twiddles = fft::get_inv_twiddles(root, cycle_length);

    let domain_size = cycle_length * blowup_factor;
    let domain_root = FieldElement::get_root_of_unity(domain_size.trailing_zeros());
    let ev_twiddles = fft::get_twiddles(domain_root, domain_size);

    (inv_twiddles, ev_twiddles)
}

pub fn extend_cyclic_values(
    values: &[FieldElement],
    inv_twiddles: &[FieldElement],
    ev_twiddles: &[FieldElement],
) -> (Vec<FieldElement>, Vec<FieldElement>) {
    let domain_size = ev_twiddles.len() * 2;
    let cycle_length = values.len();

    let mut extended_values = filled_vector(cycle_length, domain_size, FieldElement::ZERO);
    extended_values.copy_from_slice(values);
    fft::interpolate_poly(&mut extended_values, &inv_twiddles, true);

    let poly = extended_values.clone();

    unsafe {
        extended_values.set_len(extended_values.capacity());
    }
    fft::evaluate_poly(&mut extended_values, &ev_twiddles, true);

    (poly, extended_values)
}

// OTHER FUNCTIONS
// ================================================================================================

/// Transposes columns into rows in a 2-dimensional matrix.
pub fn transpose(values: Vec<Vec<FieldElement>>) -> Vec<Vec<FieldElement>> {
    let mut result = Vec::new();

    let columns = values.len();
    assert!(columns > 0, "matrix must contain at least one column");

    let rows = values[0].len();
    assert!(rows > 0, "matrix must contain at least one row");

    for _ in 0..rows {
        result.push(vec![FieldElement::ZERO; columns]);
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

/// Prints out an execution trace.
pub fn print_trace(trace: &[Vec<FieldElement>]) {
    let trace_width = trace.len();
    let trace_length = trace[0].len();

    let mut state = vec![FieldElement::ZERO; trace_width];
    for i in 0..trace_length {
        for j in 0..trace_width {
            state[j] = trace[j][i];
        }
        println!(
            "{}\t{:?}",
            i,
            state.iter().map(|v| v.as_u128()).collect::<Vec<u128>>()
        );
    }
}

/// Converts a slice of field elements values into a vector of bytes.
pub fn to_byte_vec(values: &[FieldElement]) -> Vec<u8> {
    let mut result = Vec::with_capacity(values.len() * 16);
    for value in values {
        result.extend_from_slice(&value.to_bytes());
    }
    result
}
