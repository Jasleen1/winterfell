// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

// use super::{
//     rescue, SIG_CYCLE_LENGTH as SIG_CYCLE_LEN, TRACE_WIDTH,
// };
use crate::utils::{are_equal, is_binary, is_zero, not, EvaluationResult};
use winterfell::{
    math::{fields::f128::BaseElement, FieldElement, StarkField, log2},
    Air, AirContext, Assertion, ByteWriter, EvaluationFrame, ProofOptions, Serializable, TraceInfo,
    TransitionConstraintDegree,
};

// CONSTANTS
// ================================================================================================
const TWO: BaseElement = BaseElement::new(2);

// AGGREGATE LAMPORT PLUS SIGNATURE AIR
// ================================================================================================

#[derive(Clone)]
pub struct PublicInputs {
    coefficients: Vec<BaseElement>,
    omega: BaseElement,
    degree: usize,
    output_evals: Vec<BaseElement>,
}

impl Serializable for PublicInputs {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let deg_as_u128: u128 = self.degree.try_into().unwrap();
        let deg_as_base_elt = BaseElement::from(deg_as_u128);
        target.write(&self.coefficients);
        target.write(self.omega);
        target.write(deg_as_base_elt);
        target.write(&self.output_evals);

    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.get_size_hint());
        self.write_into(&mut result);
        result
    }

    fn write_batch_into<W: ByteWriter>(source: &[Self], target: &mut W) {
        for item in source {
            item.write_into(target);
        }
    }

    fn get_size_hint(&self) -> usize {
        self.coefficients.len() + self.output_evals.len() + (2 * 128)
    }
}

pub struct FFTAir {
    context: AirContext<BaseElement>,
    coefficients: Vec<BaseElement>,
    omega: BaseElement,
    degree: usize,
}

impl Air for FFTAir {
    type BaseField = BaseElement;
    type PublicInputs = PublicInputs;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(trace_info: TraceInfo, pub_inputs: PublicInputs, options: ProofOptions) -> Self {
        // define degrees for all transition constraints
        let mut degrees = Vec::new();

        // let degrees = vec![
        //     TransitionConstraintDegree::with_cycles(2, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]), // m0 bit is binary
        //     TransitionConstraintDegree::with_cycles(2, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]), // m1 bit is binary
        //     TransitionConstraintDegree::with_cycles(
        //         1,
        //         vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN, SIG_CYCLE_LEN],
        //     ), // m0 accumulation
        //     TransitionConstraintDegree::with_cycles(
        //         1,
        //         vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN, SIG_CYCLE_LEN],
        //     ), // m1 accumulation
        //     // secret key 1 hashing
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     // secret key 2 hashing
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     // public key hashing
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        //     TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN, SIG_CYCLE_LEN]),
        // ];
        // assert_eq!(TRACE_WIDTH, trace_info.width());
        FFTAir {
            context: AirContext::new(trace_info, degrees, options),
            coefficients: pub_inputs.coefficients,
            omega: pub_inputs.omega,
            degree: pub_inputs.degree,
        }
    }

    fn context(&self) -> &AirContext<Self::BaseField> {
        &self.context
    }

    fn evaluate_transition<E: FieldElement + From<Self::BaseField>>(
        &self,
        frame: &EvaluationFrame<E>,
        periodic_values: &[E],
        result: &mut [E],
    ) {
        let current = frame.current();
        let next = frame.next();
        // TODO
        // debug_assert_eq!(TRACE_WIDTH, current.len());
        // debug_assert_eq!(TRACE_WIDTH, next.len());

        enforce_round(result, current, next, self.degree)
    }

    fn get_assertions(&self) -> Vec<Assertion<Self::BaseField>> {
        let last_cycle_step = log2(self.degree).try_into().unwrap();
        
        let mut assertions = vec![
            // the second to last register is always omega
            Assertion::periodic(self.degree, 0, 2, self.omega),
        ];
        for step_number in 0..last_cycle_step {
            // the last register is omega^{degree/(1 << step number)}
            assertions.push(Assertion::single(self.degree + 1, 
                last_cycle_step, 
                self.omega.exp((self.degree/(1 << step_number)).try_into().unwrap())));
        }
        assertions

    }

}

// HELPER FUNCTIONS
// ================================================================================================

fn enforce_round<E: FieldElement + From<BaseElement>>(
    result: &mut [E],
    current: &[E],
    next: &[E],
    degree: usize,
) {
    let step = current[degree + 2];
    let jump = 1 << step;
    let gap = (1 << (step - 1));
    let curr_omega = current[degree + 1];
    for pos in 0..degree {
        let j = pos % jump;
        result[pos] = are_equal(next[pos], current[pos] + 
            (curr_omega.exp(j) * current[pos + gap]));
        result[pos + gap] = are_equal(next[pos + gap], current[pos] - 
            (curr_omega.exp(j) * current[pos + gap]));
    }
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
//     result.agg_constraint(0, flag, are_equal(current[0], next[0]));
//     result.agg_constraint(1, flag, are_equal(current[1], next[1]));
//     result.agg_constraint(2, flag, are_equal(current[2], next[2]));
//     result.agg_constraint(3, flag, are_equal(current[3], next[3]));
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
//     result.agg_constraint(2, flag, are_equal(next_m0, next[2]));
//     let next_m1 = current[3] + current[1] * power_of_two;
//     result.agg_constraint(3, flag, are_equal(next_m1, next[3]));

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
//     result.agg_constraint(12, flag, are_equal(current[20], next[20]));
//     result.agg_constraint(13, flag, are_equal(current[21], next[21]));

//     // when current bit of m0 = 1, hash of private key 1 (which should be equal to public key)
//     // should be injected into the hasher state for public key aggregator
//     let m0_bit = current[0];
//     result.agg_constraint(14, flag * m0_bit,are_equal(current[16] + current[4], next[16]));
//     result.agg_constraint(15, flag * m0_bit, are_equal(current[17] + current[5], next[17]));

//     // when current bit of m1 = 1, hash of private key 2 (which should be equal to public key)
//     // should be injected into the hasher state for public key aggregator
//     let m1_bit = current[1];
//     result.agg_constraint(16, flag * m1_bit, are_equal(current[18] + current[10], next[18]));
//     result.agg_constraint(17, flag * m1_bit, are_equal(current[19] + current[11], next[19]));
// }

