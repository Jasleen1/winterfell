use common::utils::filled_vector;
use prover::math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
};

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
pub fn build_cyclic_domain(
    cycle_length: usize,
    blowup_factor: usize,
) -> (Vec<BaseElement>, Vec<BaseElement>) {
    let root = BaseElement::get_root_of_unity(cycle_length.trailing_zeros());
    let inv_twiddles = fft::get_inv_twiddles(root, cycle_length);

    let domain_size = cycle_length * blowup_factor;
    let domain_root = BaseElement::get_root_of_unity(domain_size.trailing_zeros());
    let ev_twiddles = fft::get_twiddles(domain_root, domain_size);

    (inv_twiddles, ev_twiddles)
}

pub fn extend_cyclic_values(
    values: &[BaseElement],
    inv_twiddles: &[BaseElement],
    ev_twiddles: &[BaseElement],
) -> (Vec<BaseElement>, Vec<BaseElement>) {
    let domain_size = ev_twiddles.len() * 2;
    let cycle_length = values.len();

    let mut extended_values = filled_vector(cycle_length, domain_size, BaseElement::ZERO);
    extended_values.copy_from_slice(values);
    fft::interpolate_poly(&mut extended_values, &inv_twiddles, true);

    let poly = extended_values.clone();

    unsafe {
        extended_values.set_len(extended_values.capacity());
    }
    fft::evaluate_poly(&mut extended_values, &ev_twiddles, true);

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
pub fn print_trace(trace: &[Vec<BaseElement>], multiples_of: usize) {
    let trace_width = trace.len();
    let trace_length = trace[0].len();

    let mut state = vec![BaseElement::ZERO; trace_width];
    for i in 0..trace_length {
        if i % multiples_of != 0 {
            continue;
        }
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
