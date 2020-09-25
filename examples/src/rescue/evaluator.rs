use common::utils::filled_vector;
use prover::{
    math::{
        fft,
        field::{self, sub, add},
    },
    ProofContext, TransitionEvaluator,
};

use super::{
    rescue::{apply_inv_mds, apply_mds, apply_sbox, ARK},
    CYCLE_LENGTH,
};

const STATE_WIDTH: usize = 4;

// RESCUE TRANSITION CONSTRAINT EVALUATOR
// ================================================================================================

pub struct RescueEvaluator {
    constraint_degrees: Vec<usize>,
    composition_coefficients: Vec<u128>,
    ark_constants: Vec<Vec<u128>>,
    ark_polys: Vec<Vec<u128>>,
}

impl TransitionEvaluator for RescueEvaluator {
    const MAX_CONSTRAINTS: usize = 4;
    const MAX_CONSTRAINT_DEGREE: usize = 4;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(context: &ProofContext, coefficients: &[u128]) -> Self {
        let constraint_degrees = vec![3, 3, 3, 3];
        let composition_coefficients = coefficients[..8].to_vec();
        let (ark_polys, ark_constants) = extend_constants(context.ce_blowup_factor());

        RescueEvaluator {
            constraint_degrees,
            composition_coefficients,
            ark_constants,
            ark_polys,
        }
    }

    // TRANSITION CONSTRAINTS
    // --------------------------------------------------------------------------------------------

    fn evaluate(&self, current: &[u128], next: &[u128], step: usize) -> Vec<u128> {

        let ark = &self.ark_constants[step % self.ark_constants.len()];

        let mut current = current.to_vec();
        apply_sbox(&mut current);
        apply_mds(&mut current);
        for i in 0..STATE_WIDTH {
            current[i] = add(current[i], ark[i]);
        }

        let mut next = next.to_vec();
        for i in 0..STATE_WIDTH {
            next[i] = sub(next[i], ark[STATE_WIDTH + i]);
        }
        apply_inv_mds(&mut next);
        apply_sbox(&mut next);

        are_equal(current, next)
    }

    fn evaluate_at(&self, current: &[u128], next: &[u128], _x: u128) -> Vec<u128> {
        let ark = vec![];

        let mut current = current.to_vec();
        apply_sbox(&mut current);
        apply_mds(&mut current);
        for i in 0..STATE_WIDTH {
            current[i] = add(current[i], ark[i]);
        }

        let mut next = next.to_vec();
        for i in 0..STATE_WIDTH {
            next[i] = sub(next[i], ark[STATE_WIDTH + i]);
        }
        apply_inv_mds(&mut next);
        apply_sbox(&mut next);

        are_equal(current, next)
    }

    // BOILERPLATE
    // --------------------------------------------------------------------------------------------
    fn degrees(&self) -> &[usize] {
        &self.constraint_degrees
    }

    fn composition_coefficients(&self) -> &[u128] {
        &self.composition_coefficients
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn are_equal(mut a: Vec<u128>, b: Vec<u128>) -> Vec<u128> {
    for i in 0..a.len() {
        a[i] = sub(a[i], b[i]);
    }
    a
}

fn extend_constants(blowup_factor: usize) -> (Vec<Vec<u128>>, Vec<Vec<u128>>) {
    let constants = transpose_ark();

    let root = field::get_root_of_unity(CYCLE_LENGTH);
    let inv_twiddles = fft::get_inv_twiddles(root, CYCLE_LENGTH);

    let domain_size = CYCLE_LENGTH * blowup_factor;
    let domain_root = field::get_root_of_unity(domain_size);
    let twiddles = fft::get_twiddles(domain_root, domain_size);

    let mut polys = Vec::with_capacity(constants.len());
    let mut evaluations = Vec::with_capacity(constants.len());

    for constant in constants.iter() {
        let mut extended_constant = filled_vector(CYCLE_LENGTH, domain_size, field::ZERO);
        extended_constant.copy_from_slice(constant);

        fft::interpolate_poly(&mut extended_constant, &inv_twiddles, true);
        polys.push(extended_constant.clone());

        unsafe {
            extended_constant.set_len(extended_constant.capacity());
        }
        fft::evaluate_poly(&mut extended_constant, &twiddles, true);

        evaluations.push(extended_constant);
    }

    evaluations = transpose(evaluations);

    (polys, evaluations)
}

fn transpose_ark() -> Vec<Vec<u128>> {
    let mut constants = Vec::new();
    for _ in 0..(STATE_WIDTH * 2) {
        constants.push(vec![0; CYCLE_LENGTH]);
    }

    for i in 0..CYCLE_LENGTH {
        for j in 0..(STATE_WIDTH * 2) {
            constants[j][i] = ARK[i][j];
        }
    }

    constants
}

fn transpose(values: Vec<Vec<u128>>) -> Vec<Vec<u128>> {
    let mut result = Vec::new();

    let width = values.len();
    let length = values[0].len();

    for _ in 0..length {
        result.push(vec![0; width]);
    }

    for i in 0..width {
        for j in 0..length {
            result[j][i] = values[i][j];
        }
    }

    result
}
