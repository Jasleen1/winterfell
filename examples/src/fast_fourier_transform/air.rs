// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::vec;

// use super::{
//     rescue, SIG_CYCLE_LENGTH as SIG_CYCLE_LEN, TRACE_WIDTH,
// };
use crate::utils::{are_equal, fast_fourier_transform::bit_reverse};
use winterfell::{
    math::{fields::f128::BaseElement, log2, FieldElement},
    Air, AirContext, Assertion, ByteWriter, EvaluationFrame, ProofOptions, Serializable, TraceInfo,
    TransitionConstraintDegree,
};

use super::prover::{
    get_num_main_trace_rows_u64, get_num_trace_rows, get_power_of_two_pos, get_rev_counter_inv_pos,
    get_rev_counter_pos,
};

// CONSTANTS
// ================================================================================================
// const TWO: BaseElement = BaseElement::new(2);

// AGGREGATE LAMPORT PLUS SIGNATURE AIR
// ================================================================================================

#[derive(Clone)]
pub struct PublicInputs {
    pub num_inputs: usize,
    pub fft_inputs: Vec<BaseElement>,
    pub result: Vec<BaseElement>,
}

impl Serializable for PublicInputs {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(&self.result[..]);
    }
}

pub struct FFTAir {
    context: AirContext<BaseElement>,
    fft_inputs: Vec<BaseElement>,
    result: Vec<BaseElement>,
}

impl Air for FFTAir {
    type BaseField = BaseElement;
    type PublicInputs = PublicInputs;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(trace_info: TraceInfo, pub_inputs: PublicInputs, options: ProofOptions) -> Self {
        // define degrees for all transition constraints
        let mut degrees = Vec::new();
        // TODO
        // let enforce_permutation_deg = TransitionConstraintDegree::new(2);
        // for _ in 0..pub_inputs.num_inputs {
        //     degrees.push(enforce_permutation_deg.clone());
        // }

        let log_deg_usize: usize = log2(pub_inputs.num_inputs).try_into().unwrap();
        let num_fft_steps = log_deg_usize + 1;

        for step_count in (num_fft_steps - 1)..num_fft_steps {
            let m = 1 << step_count;
            let jump = 1 << (step_count - 1);
            let num_ranges: usize = pub_inputs.num_inputs / m;
            let mut local_constraint_degs =
                vec![TransitionConstraintDegree::new(2); pub_inputs.num_inputs];
            for k in 0..num_ranges {
                let start_of_range = k * 2 * jump;
                for j in 0..jump {
                    local_constraint_degs[start_of_range + j] =
                        TransitionConstraintDegree::new(2 + 2 + j);
                    local_constraint_degs[start_of_range + j + jump] =
                        TransitionConstraintDegree::new(2 + 2 + j);
                }
            }
            degrees.append(&mut local_constraint_degs);
        }

        // for step in 1..trace_usize {
        //     for counter in (0..pub_inputs.num_inputs).step_by(1<<step) {
        //         for _ in 0..(1<<(step - 1)) {
        //             let enforce_butterfly_deg_1 = TransitionConstraintDegree::new(1);
        //             let enforce_butterfly_deg_2 = TransitionConstraintDegree::new(counter/(1<<step) + 1);
        //             let enforce_butterfly_deg_3 = TransitionConstraintDegree::new(counter/(1<<step) + 1);
        //             degrees.push(enforce_butterfly_deg_1);
        //             degrees.push(enforce_butterfly_deg_2);
        //             degrees.push(enforce_butterfly_deg_3);
        //         }
        //     }
        // }

        let omega_deg = TransitionConstraintDegree::new(4);
        degrees.push(omega_deg);

        let power_of_two_deg = TransitionConstraintDegree::new(3);
        degrees.push(power_of_two_deg);

        let dummy_selector_deg = TransitionConstraintDegree::new(2);
        degrees.push(dummy_selector_deg);

        // let step_count_deg = TransitionConstraintDegree::new(1);
        // degrees.push(step_count_deg);

        // for _ in 0..log_deg_usize {
        //     let enforce_selector_deg = TransitionConstraintDegree::new(1);
        //     degrees.push(enforce_selector_deg);
        // }
        // assert_eq!(TRACE_WIDTH, trace_info.width());
        FFTAir {
            context: AirContext::new(trace_info, degrees, options),
            fft_inputs: pub_inputs.fft_inputs,
            result: pub_inputs.result,
        }
    }

    fn context(&self) -> &AirContext<Self::BaseField> {
        &self.context
    }

    fn evaluate_transition<E: FieldElement + From<Self::BaseField>>(
        &self,
        frame: &EvaluationFrame<E>,
        _periodic_values: &[E],
        result: &mut [E],
    ) {
        let current = frame.current();
        let next = frame.next();
        // TODO
        // debug_assert_eq!(TRACE_WIDTH, current.len());
        // debug_assert_eq!(TRACE_WIDTH, next.len());
        let num_inputs = self.fft_inputs.len();
        let reverse_perm = get_reverse_permutation(num_inputs);
        enforce_round(result, current, next, num_inputs, reverse_perm);
    }

    fn get_assertions(&self) -> Vec<Assertion<Self::BaseField>> {
        let mut assertions = vec![];
        let num_fft_inputs = self.fft_inputs.len();
        let full_trace_length = get_num_trace_rows(num_fft_inputs);
        let log_num_fft_inputs: usize = log2(num_fft_inputs).try_into().unwrap();
        let log_num_fft_inputs_u64: u64 = get_num_main_trace_rows_u64(num_fft_inputs);
        let power_of_two_register = get_power_of_two_pos(num_fft_inputs);
        assertions.push(Assertion::single(
            power_of_two_register,
            0,
            BaseElement::from(2u64),
        ));

        let rev_counter_pos = get_rev_counter_pos(num_fft_inputs, log_num_fft_inputs);
        assertions.push(Assertion::single(
            rev_counter_pos,
            0,
            BaseElement::from(log_num_fft_inputs_u64),
        ));
        let rev_counter_inv_pos = get_rev_counter_inv_pos(num_fft_inputs, log_num_fft_inputs);
        assertions.push(Assertion::single(
            rev_counter_inv_pos,
            0,
            BaseElement::from(log_num_fft_inputs_u64).inv(),
        ));
        for i in 0..num_fft_inputs {
            assertions.push(Assertion::single(i, full_trace_length - 1, self.result[i]));
        }
        // assertions.push(
        //     Assertion::single(power_of_two_register, 1, BaseElement::from(2u64))
        // );
        // let log_degree: usize = log2(self.degree).try_into().unwrap();
        // let total_steps = log_degree + 1;
        // let mut assertions = vec![
        //     // the register at index data_size is always omega
        //     Assertion::periodic(self.degree, 0, 2, self.omega),
        //     Assertion::periodic(self.degree, 1, 2, self.omega),
        // ];
        // for step_number in 0..total_steps {
        //     // this register is omega^{degree/(1 << step number)}
        //     assertions.push(Assertion::single(self.degree + 1,
        //         step_number,
        //         self.omega.exp((self.degree/(1 << step_number)).try_into().unwrap())));
        // }
        // // The (degree + 2)th register contains the step number
        // assertions.push(Assertion::single(self.degree + 2,
        //     0,
        //     BaseElement::ZERO));

        // // These registers hold i - step_number for each possible step i.
        // for step_number in 0..total_steps {
        //     let step_u128: u128 = if step_number == 0 {1} else {0};
        //     assertions.push(Assertion::single(get_selector_pos(step_number, self.degree),
        //         0,
        //         BaseElement::from(step_u128)));
        // }

        // FIXME: Need one more constraint to check the inverses
        // are correct but that may be possible to put in evaluation constraints.

        // // These registers hold i - step_number for each possible step i,
        // // so when step_number = the position, the value should be zero.
        // for step_number in 0..last_cycle_step {
        //     assertions.push(Assertion::single(get_count_diff_pos(step_number, self.degree),
        //         step_number,
        //         BaseElement::ZERO));
        //     assertions.push(Assertion::single(get_selector_pos(step_number, self.degree),
        //         step_number,
        //         BaseElement::ONE));
        // }

        assertions
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn enforce_round<E: FieldElement + From<BaseElement>>(
    result: &mut [E],
    current: &[E],
    next: &[E],
    num_fft_inputs: usize,
    reverse_perm: Vec<usize>,
) {
    // FFT part
    for pos in 0..num_fft_inputs {
        result[pos] = E::ZERO;
    }
    let log_num_fft_inputs: usize = log2(num_fft_inputs).try_into().unwrap();

    let rev_counter_pos = get_rev_counter_pos(num_fft_inputs, log_num_fft_inputs);
    let rev_counter_inv_pos = get_rev_counter_inv_pos(num_fft_inputs, log_num_fft_inputs);

    let handle_bit = next[rev_counter_inv_pos] * next[rev_counter_pos];

    result[num_fft_inputs + 2] = are_equal(
        next[rev_counter_pos],
        current[rev_counter_pos] * (E::ONE - current[rev_counter_inv_pos]),
    );

    enforce_0th_round(
        result,
        current,
        next,
        num_fft_inputs,
        reverse_perm,
        handle_bit,
    );
    enforce_butterfly_round(result, current, next, num_fft_inputs, handle_bit);
    // result[num_fft_inputs] = are_equal(current[num_fft_inputs], next[num_fft_inputs]);
    let curr_omega = current[num_fft_inputs];
    let next_omega = next[num_fft_inputs];
    let power_of_two_pos = get_power_of_two_pos(num_fft_inputs);
    result[num_fft_inputs] =
        handle_bit * are_equal(curr_omega, next_omega.exp(2u32.try_into().unwrap()));
    result[num_fft_inputs + 1] = handle_bit
        * are_equal(
            current[power_of_two_pos] * E::from(2u64),
            next[power_of_two_pos],
        );

    // r(X) = a(X) - b(omega * X) and if constraint satisfied then this should be 0 on the eval domain.
    // # of points on which r is interpol = trace_len - 1
    // Deg(r) = deg(RHS)
    // If deg(r) <= trace_len - 1, then if r = 0 on trace_len - 1 points, r must be the zero poly.
    // r'(X) = (a(X))^2 - (b(omega * X))^2 = (a(X) - b(X)) * (a(X) + b(X))
    // deg(r') = 2 * max(deg(a), deg(b))
    // result[num_fft_inputs + 2] = are_equal(current[num_fft_inputs + 2] + E::ONE, next[num_fft_inputs + 2]);
    // // Auxiliary parts
    // let log_degree = log2(num_fft_inputs).try_into().unwrap();
    // for i in 0..log_degree+1 {
    //     let selector_pos = get_selector_pos(i, num_fft_inputs);
    //     result[selector_pos] = are_equal(next[selector_pos], current[get_previous_selector_pos(i, num_fft_inputs, log_degree)]);
    //     // result[selector_pos] = are_equal(next[selector_pos], current[get_previous_selector_pos(i, data_size, log_degree)]);
    // }

    // for i in 0..num_fft_inputs {
    //     result[i] = not(result[i]);
    // }
}

fn enforce_0th_round<E: FieldElement + From<BaseElement>>(
    result: &mut [E],
    current: &[E],
    next: &[E],
    num_fft_inputs: usize,
    reverse_perm: Vec<usize>,
    handle_bit: E,
) {
    let selector = handle_bit * current[get_selector_pos(0, num_fft_inputs)];

    for i in 0..num_fft_inputs {
        result[i] = selector * are_equal(next[i], current[reverse_perm[i]]);
    }
}

fn enforce_butterfly_round<E: FieldElement + From<BaseElement>>(
    result: &mut [E],
    current: &[E],
    next: &[E],
    num_fft_inputs: usize,
    handle_bit: E,
) {
    // let step = current[degree + 2];

    let num_steps: usize = (log2(num_fft_inputs) + 1).try_into().unwrap();
    for step in 1..num_steps {
        let selector = handle_bit * current[get_selector_pos(step, num_fft_inputs)];

        let curr_omega = next[num_fft_inputs];

        let m = 1 << step;
        let jump = 1 << (step - 1);
        let num_ranges: usize = num_fft_inputs / m;
        for k in 0..num_ranges {
            // let mut running_omega = E::ONE;
            let start_of_range = k * 2 * jump;
            for j_usize in 0..jump {
                let j_u64: u64 = j_usize.try_into().unwrap();
                // let jump = 1 << step;
                // let gap = 1 << (step - 1);
                // let j_64: u64 = (pos % jump).try_into().unwrap();

                // let j = E::PositiveInteger::from(j_usize.try_into().unwrap());
                let u = current[start_of_range + j_usize];
                let v = current[start_of_range + j_usize + jump]
                    * (curr_omega.exp(E::PositiveInteger::from(j_u64)));
                result[start_of_range + j_usize] +=
                    selector * (are_equal(next[start_of_range + j_usize], u + v));
                // current[start_of_range + j_usize]
                // + (current[start_of_range + j_usize + jump] * (curr_omega.exp(E::PositiveInteger::from(j_u64))))));
                result[start_of_range + j_usize + jump] +=
                    selector * are_equal(next[start_of_range + j_usize + jump], u - v)
                // running_omega = running_omega * curr_omega;
            }
        }
    }
}

fn get_selector_pos(i: usize, num_fft_inputs: usize) -> usize {
    num_fft_inputs + 2 + i
}

// #[test]
// fn test_get_selector_pos() {
//     assert!(get_selector_pos(0, 8) == 11);
//     assert!(get_selector_pos(1, 8) == 12);
//     assert!(get_selector_pos(0, 16) == 19);
//     assert!(get_selector_pos(1, 16) == 20);
// }

// fn get_selector_pos(i: usize, degree: usize) -> usize {
// degree + 3 + 3*i + 2
// }

fn get_reverse_permutation(size: usize) -> Vec<usize> {
    let log_size = log2(size);
    let mut permutation_vec = Vec::new();
    for i in 0..size {
        permutation_vec.push(i);
    }
    for i in 0..size {
        let rev = bit_reverse(i, log_size.try_into().unwrap());
        if i < rev {
            swap(i, rev, &mut permutation_vec);
        }
    }
    permutation_vec
}

fn swap<T: Copy>(pos1: usize, pos2: usize, state: &mut [T]) {
    state.swap(pos1, pos2)
}

// #[rustfmt::skip]
// fn evaluate_constraints<E: FieldElement + From<BaseElement>>(
//     result: &mut [E],
//     current: &[E],
//     next: &[E],
//     ark: &[E],
//     hash_flag: E,
//     sig_cycle_end_flag: E,
//     power_of_two: E,
// ) {
//     // when hash_flag = 1 (which happens on all steps except steps which are one less than a
//     // multiple of 8 - e.g. all steps except for 7, 15, 23 etc.), and we are not on the last step
//     // of a signature cycle make sure the contents of the first 4 registers are copied over, and
//     // for other registers, Rescue constraints are applied separately for hashing secret and
//     // public keys
//     let flag = not(sig_cycle_end_flag) * hash_flag;
//     result.agg_constraint(0, flag, not(are_equal(current[0], next[0]));
//     result.agg_constraint(1, flag, not(are_equal(current[1], next[1]));
//     result.agg_constraint(2, flag, not(are_equal(current[2], next[2]));
//     result.agg_constraint(3, flag, not(are_equal(current[3], next[3]));
//     rescue::enforce_round(&mut result[4..10],  &current[4..10],  &next[4..10],  ark, flag);
//     rescue::enforce_round(&mut result[10..16], &current[10..16], &next[10..16], ark, flag);
//     rescue::enforce_round(&mut result[16..22], &current[16..22], &next[16..22], ark, flag);

//     // when hash_flag = 0 (which happens on steps which are one less than a multiple of 8 - e.g. 7,
//     // 15, 23 etc.), and we are not on the last step of a signature cycle:
//     let flag = not(sig_cycle_end_flag) * not(hash_flag);
//     // make sure values inserted into registers 0 and 1 are binary
//     result.agg_constraint(0, flag, is_binary(current[0]));
//     result.agg_constraint(1, flag, is_binary(current[1]));
//     // make sure message values were aggregated correctly in registers 2 and 3
//     let next_m0 = current[2] + current[0] * power_of_two;
//     result.agg_constraint(2, flag, not(are_equal(next_m0, next[2]));
//     let next_m1 = current[3] + current[1] * power_of_two;
//     result.agg_constraint(3, flag, not(are_equal(next_m1, next[3]));

//     // registers 6..10 and 12..16 were set to zeros
//     result.agg_constraint(4, flag, is_zero(next[6]));
//     result.agg_constraint(5, flag, is_zero(next[7]));
//     result.agg_constraint(6, flag, is_zero(next[8]));
//     result.agg_constraint(7, flag, is_zero(next[9]));
//     result.agg_constraint(8, flag, is_zero(next[12]));
//     result.agg_constraint(9, flag, is_zero(next[13]));
//     result.agg_constraint(10, flag, is_zero(next[14]));
//     result.agg_constraint(11, flag, is_zero(next[15]));

//     // contents of registers 20 and 21 (capacity section of public key hasher state) were
//     // copied over to the next step
//     result.agg_constraint(12, flag, not(are_equal(current[20], next[20]));
//     result.agg_constraint(13, flag, not(are_equal(current[21], next[21]));

//     // when current bit of m0 = 1, hash of private key 1 (which should be equal to public key)
//     // should be injected into the hasher state for public key aggregator
//     let m0_bit = current[0];
//     result.agg_constraint(14, flag * m0_bit,not(are_equal(current[16] + current[4], next[16]));
//     result.agg_constraint(15, flag * m0_bit, not(are_equal(current[17] + current[5], next[17]));

//     // when current bit of m1 = 1, hash of private key 2 (which should be equal to public key)
//     // should be injected into the hasher state for public key aggregator
//     let m1_bit = current[1];
//     result.agg_constraint(16, flag * m1_bit, not(are_equal(current[18] + current[10], next[18]));
//     result.agg_constraint(17, flag * m1_bit, not(are_equal(current[19] + current[11], next[19]));
// }
