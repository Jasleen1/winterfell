use math::field::StarkField;
use prover::math::{
    fft,
    field::{BaseElement, FieldElement},
};
use std::ops::Range;

pub mod rescue;

// CONSTRAINT EVALUATION HELPERS
// ================================================================================================

pub fn are_equal<E: FieldElement>(a: E, b: E) -> E {
    a - b
}

pub fn is_zero<E: FieldElement>(a: E) -> E {
    a
}

pub fn is_binary<E: FieldElement>(a: E) -> E {
    a * a - a
}

pub fn not<E: FieldElement>(a: E) -> E {
    E::ONE - a
}

pub fn when<E: FieldElement>(a: E, b: E) -> E {
    a * b
}

// TRAIT TO SIMPLIFY CONSTRAINT AGGREGATION
// ================================================================================================

pub trait EvaluationResult<E> {
    fn agg_constraint(&mut self, index: usize, flag: E, value: E);
}

impl<E: FieldElement> EvaluationResult<E> for [E] {
    fn agg_constraint(&mut self, index: usize, flag: E, value: E) {
        self[index] = self[index] + flag * value;
    }
}

impl<E: FieldElement> EvaluationResult<E> for Vec<E> {
    fn agg_constraint(&mut self, index: usize, flag: E, value: E) {
        self[index] = self[index] + flag * value;
    }
}

// CYCLIC VALUES
// ================================================================================================

/// Builds extension domain for cyclic registers.
pub fn build_cyclic_domain(cycle_length: usize) -> (Vec<BaseElement>, Vec<BaseElement>) {
    let inv_twiddles = fft::get_inv_twiddles(cycle_length);
    let ev_twiddles = fft::get_twiddles(cycle_length);
    (inv_twiddles, ev_twiddles)
}

pub fn extend_cyclic_values(
    values: &[BaseElement],
    inv_twiddles: &[BaseElement],
    ev_twiddles: &[BaseElement],
    blowup_factor: usize,
    trace_length: usize,
) -> (Vec<BaseElement>, Vec<BaseElement>) {
    let num_cycles = (trace_length / values.len()) as u64;

    let mut poly = values.to_vec();
    fft::interpolate_poly(&mut poly, &inv_twiddles);

    let offset = BaseElement::GENERATOR.exp(num_cycles.into());
    let extended_values =
        fft::evaluate_poly_with_offset(&poly, &ev_twiddles, offset, blowup_factor);

    (poly, extended_values)
}

// MERKLE TREE FUNCTIONS
// ================================================================================================

pub type TreeNode = (BaseElement, BaseElement);

pub fn node_to_bytes(node: TreeNode) -> [u8; 32] {
    let mut result = [0; 32];
    BaseElement::write_into(&[node.0, node.1], &mut result).unwrap();
    result
}

pub fn bytes_to_node(bytes: [u8; 32]) -> TreeNode {
    let elements = BaseElement::read_to_vec(&bytes).unwrap();
    (elements[0], elements[1])
}

// OTHER FUNCTIONS
// ================================================================================================

/// Transposes columns into rows in a 2-dimensional matrix.
pub fn transpose(values: Vec<Vec<BaseElement>>) -> Vec<Vec<BaseElement>> {
    let mut result = Vec::new();

    let columns = values.len();
    assert!(columns > 0, "matrix must contain at least one column");

    let rows = values[0].len();
    assert!(rows > 0, "matrix must contain at least one row");

    for _ in 0..rows {
        result.push(vec![BaseElement::ZERO; columns]);
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
pub fn print_trace(
    trace: &[Vec<BaseElement>],
    multiples_of: usize,
    offset: usize,
    range: Range<usize>,
) {
    let trace_width = trace.len();
    let trace_length = trace[0].len();

    let mut state = vec![BaseElement::ZERO; trace_width];
    for i in 0..trace_length {
        if (i.wrapping_sub(offset)) % multiples_of != 0 {
            continue;
        }
        for j in 0..trace_width {
            state[j] = trace[j][i];
        }
        println!(
            "{}\t{:?}",
            i,
            state[range.clone()]
                .iter()
                .map(|v| v.as_u128())
                .collect::<Vec<u128>>()
        );
    }
}

pub fn print_trace_step(trace: &[Vec<BaseElement>], step: usize) {
    let trace_width = trace.len();
    let mut state = vec![BaseElement::ZERO; trace_width];
    for i in 0..trace_width {
        state[i] = trace[i][step];
    }
    println!(
        "{}\t{:?}",
        step,
        state.iter().map(|v| v.as_u128()).collect::<Vec<u128>>()
    );
}
