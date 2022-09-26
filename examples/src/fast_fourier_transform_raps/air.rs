// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::{convert::TryInto, env::current_exe};

use super::{
    rescue::{self, STATE_WIDTH},
    BaseElement, ExtensionOf, FieldElement, ProofOptions, CYCLE_LENGTH, TRACE_WIDTH,
};
use crate::utils::{are_equal, not, EvaluationResult};
use winterfell::{
    Air, AirContext, Assertion, AuxTraceRandElements, ByteWriter, EvaluationFrame, Serializable,
    TraceInfo, TransitionConstraintDegree, math::{StarkField, log2, fft},
};

// CONSTANTS
// ================================================================================================

/// Specifies steps on which Rescue transition function is applied.
const CYCLE_MASK: [BaseElement; CYCLE_LENGTH] = [
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ZERO,
    BaseElement::ZERO,
];

// RESCUE AIR
// ================================================================================================

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

pub struct FFTRapsAir {
    context: AirContext<BaseElement>,
    fft_inputs: Vec<BaseElement>,
    result: Vec<BaseElement>,
}

impl Air for FFTRapsAir {
    type BaseField = BaseElement;
    type PublicInputs = PublicInputs;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(trace_info: TraceInfo, pub_inputs: PublicInputs, options: ProofOptions) -> Self {
        let main_degrees =
            vec![TransitionConstraintDegree::new(1); pub_inputs.fft_inputs.len()-1];
        // println!("Main degrees = {:?}", main_degrees);
        let aux_degrees = vec![];
        // let aux_degrees = vec![
        //     TransitionConstraintDegree::new(1);
        //     (pub_inputs.fft_inputs.len()-3)/2
        // ];
        // let log_num_inputs: usize = log2(pub_inputs.fft_inputs.len()).try_into().unwrap();
        // assert_eq!(2*log_num_inputs + 3, trace_info.width());
        // FFTRapsAir {
        //     context: AirContext::new_multi_segment(
        //         trace_info,
        //         main_degrees,
        //         aux_degrees,
        //         2*pub_inputs.fft_inputs.len(),
        //         pub_inputs.fft_inputs.len()-3,
        //         options,
        //     ),
        //     fft_inputs: pub_inputs.fft_inputs,
        //     result: pub_inputs.result,
        // }
        FFTRapsAir {
            context: AirContext::new_multi_segment(
                trace_info,
                main_degrees,
                aux_degrees,
                1,//2*pub_inputs.fft_inputs.len(),
                0,//pub_inputs.fft_inputs.len()-3,
                options,
            ),
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
        periodic_values: &[E],
        result: &mut [E],
    ) {
        let current = frame.current();
        let next = frame.next();

        debug_assert_eq!(next.len(), current.len());
        let num_steps: usize = log2(self.fft_inputs.len()).try_into().unwrap();
        // You'll actually only check constraints at even steps, at odd steps you don't do anything
        let compute_flag = periodic_values[num_steps];
        for step in 1..num_steps+1 {
            let local_omega = periodic_values[step-1];
            let u = current[2*step-1];
            let v = next[2*step-1] * local_omega;
            
            result[2*step-2] = compute_flag * (u + v - current[2*step]);
            result[2*step-1] = compute_flag * (u - v - next[2*step]);
            // println!("lstep = {}, result postions = {} and {}", step, 2*step - 2, 2*step - 1);
            // println!("local omega = {:?}", periodic_values[step-1]);
            // println!("Result {} = {:?}", 2*step-2, result[2*step - 2]);
            // println!("Result {} = {:?}", 2*step-1, result[2*step - 1]);
        }
        // println!("Result for step {:?} = {:?}", current[current.len() - 1], result);

        // // split periodic values into hash_flag, absorption flag and Rescue round constants
        // let hash_flag = periodic_values[0];
        // let absorption_flag = periodic_values[1];
        // let ark = &periodic_values[2..];

        // // when hash_flag = 1, constraints for Rescue round are enforced (steps 0 to 14)
        // rescue::enforce_round(
        //     &mut result[..STATE_WIDTH],
        //     &current[..STATE_WIDTH],
        //     &next[..STATE_WIDTH],
        //     ark,
        //     hash_flag,
        // );

        // rescue::enforce_round(
        //     &mut result[STATE_WIDTH..],
        //     &current[STATE_WIDTH..],
        //     &next[STATE_WIDTH..],
        //     ark,
        //     hash_flag,
        // );

        // // When absorbing the additional seeds (step 14), we do not verify correctness of the
        // // rate registers. Instead, we only verify that capacity registers have not
        // // changed. When computing the permutation argument, we will recompute the permuted
        // // values from the contiguous rows.
        // // At step 15, we enforce that the whole hash states are copied to the next step,
        // // enforcing that the values added to the capacity registers at step 14 and used in the
        // // permutation argument are the ones being used in the next hashing sequence.
        // result.agg_constraint(2, absorption_flag, are_equal(current[2], next[2]));
        // result.agg_constraint(3, absorption_flag, are_equal(current[3], next[3]));

        // result.agg_constraint(6, absorption_flag, are_equal(current[6], next[6]));
        // result.agg_constraint(7, absorption_flag, are_equal(current[7], next[7]));

        // // when hash_flag + absorption_flag = 0, constraints for copying hash values to the
        // // next step are enforced.
        // let copy_flag = not(hash_flag + absorption_flag);
        // enforce_hash_copy(
        //     &mut result[..STATE_WIDTH],
        //     &current[..STATE_WIDTH],
        //     &next[..STATE_WIDTH],
        //     copy_flag,
        // );
        // enforce_hash_copy(
        //     &mut result[STATE_WIDTH..],
        //     &current[STATE_WIDTH..],
        //     &next[STATE_WIDTH..],
        //     copy_flag,
        // );
    }

    fn evaluate_aux_transition<F, E>(
        &self,
        main_frame: &EvaluationFrame<F>,
        aux_frame: &EvaluationFrame<E>,
        periodic_values: &[F],
        aux_rand_elements: &AuxTraceRandElements<E>,
        result: &mut [E],
    ) where
        F: FieldElement<BaseField = Self::BaseField>,
        E: FieldElement<BaseField = Self::BaseField> + ExtensionOf<F>,
    {
        return;
        // let main_current = main_frame.current();
        // let main_next = main_frame.next();

        // let aux_current = aux_frame.current();
        // let aux_next = aux_frame.next();

        // let random_elements = aux_rand_elements.get_segment_elements(0);

        // let absorption_flag = periodic_values[1];

        // // We want to enforce that the absorbed values of the first hash chain are a
        // // permutation of the absorbed values of the second one. Because we want to
        // // copy two values per hash chain (namely the two capacity registers), we
        // // group them with random elements into a single cell via
        // // α_0 * c_0 + α_1 * c_1, where c_i is computed as next_i - current_i.

        // // Note that storing the copied values into two auxiliary columns. One could
        // // instead directly compute the permutation argument, hence require a single
        // // auxiliary one. For the sake of illustrating RAPs behaviour, we will store
        // // the computed values in additional columns.

        // let copied_value_1 = random_elements[0] * (main_next[0] - main_current[0]).into()
        //     + random_elements[1] * (main_next[1] - main_current[1]).into();

        // result.agg_constraint(
        //     0,
        //     absorption_flag.into(),
        //     are_equal(aux_current[0], copied_value_1),
        // );

        // let copied_value_2 = random_elements[0] * (main_next[4] - main_current[4]).into()
        //     + random_elements[1] * (main_next[5] - main_current[5]).into();

        // result.agg_constraint(
        //     1,
        //     absorption_flag.into(),
        //     are_equal(aux_current[1], copied_value_2),
        // );

        // // Enforce that the permutation argument column scales at each step by (aux[0] + γ) / (aux[1] + γ).
        // result.agg_constraint(
        //     2,
        //     E::ONE,
        //     are_equal(
        //         aux_next[2] * (aux_current[1] + random_elements[2]),
        //         aux_current[2] * (aux_current[0] + random_elements[2]),
        //     ),
        // );
    }

    fn get_assertions(&self) -> Vec<Assertion<Self::BaseField>> {
        // Assert starting and ending values of the hash chain
        // let last_step = self.trace_length() - 1;
        // vec![
        //     // Initial capacity registers must be set to zero
        //     Assertion::single(2, 0, BaseElement::ZERO),
        //     Assertion::single(3, 0, BaseElement::ZERO),
        //     Assertion::single(6, 0, BaseElement::ZERO),
        //     Assertion::single(7, 0, BaseElement::ZERO),
        //     // // Final rate registers (digests) should be equal to
        //     // // the provided public input
        //     // Assertion::single(0, last_step, self.result[0][0]),
        //     // Assertion::single(1, last_step, self.result[0][1]),
        //     // Assertion::single(4, last_step, self.result[1][0]),
        //     // Assertion::single(5, last_step, self.result[1][1]),
        // ]
        vec![
            Assertion::single(0, 0, self.fft_inputs[0]),
        ]
    }

    fn get_aux_assertions<E: FieldElement + From<Self::BaseField>>(
        &self,
        _aux_rand_elements: &AuxTraceRandElements<E>,
    ) -> Vec<Assertion<E>> {
        // let last_step = self.trace_length() - 1;
        // vec![
        //     Assertion::single(2, 0, E::ONE),
        //     Assertion::single(2, last_step, E::ONE),
        // ]
        vec![]
    }

    fn get_periodic_column_values(&self) -> Vec<Vec<Self::BaseField>> {
        let fft_size = self.fft_inputs.len();
        let fft_size_u128: u128 = fft_size.try_into().unwrap();
        let fft_size_u32: u32 = fft_size.try_into().unwrap();
        let num_steps: usize = log2(fft_size).try_into().unwrap();
        let mut result = Vec::<Vec::<BaseElement>>::new();
        let omega = BaseElement::get_root_of_unity(fft_size_u32);
        // println!("In the periodic col generation");
        for step in 0..num_steps {
            // println!("Step = {}", step);
            let m = 1 << (step+1);
            let m_u128: u128 = m.try_into().unwrap();
            let mut local_omega_col = vec![BaseElement::ONE; m];
            let local_omega = omega.exp(fft_size_u128/m_u128);
            for i in 0..m/2 {
                let i_u128: u128 = i.try_into().unwrap();
                local_omega_col[2*i] = local_omega.exp(i_u128);
            }
            // println!("Local omega col step {} = {:?}", step, local_omega_col);
            result.push(local_omega_col);
        }
        // println!("\n ******** \n");
        let flags = vec![BaseElement::ONE,BaseElement::ZERO];
        result.push(flags);
        result
    }
}

// HELPER EVALUATORS
// ------------------------------------------------------------------------------------------------

/// when flag = 1, enforces that the next state of the computation is defined like so:
/// - the first two registers are equal to the values from the previous step
/// - the other two registers are not restrained, they could be arbitrary elements,
///   until the RAP columns enforces that they are a permutation of the two final registers
///   of the other parallel chain
fn enforce_hash_copy<E: FieldElement>(result: &mut [E], current: &[E], next: &[E], flag: E) {
    result.agg_constraint(0, flag, are_equal(current[0], next[0]));
    result.agg_constraint(1, flag, are_equal(current[1], next[1]));
    result.agg_constraint(2, flag, are_equal(current[2], next[2]));
    result.agg_constraint(3, flag, are_equal(current[3], next[3]));
}

// fn compute_omega_for_row_and_col<E: FieldElement>(row: E, col: usize, fft_size: usize, omega: E) {
//     assert_eq!(col % 2, 1, "Only odd columns perform FFT ops");
//     let step = <E as FieldElement>::as_base_elements(&[row])[0].as_int(); 
//     assert_eq!(, 1, "Only odd rows show compute the omega");
//     let m = 1 << ((col + 1)/2);
//     let fft_size_u128: u128 = fft_size.try_into().unwrap();
//     let m = 1 << ((step + 1)/2);
//     let m_u128: u128 = m.try_into().unwrap();
//     let local_omega = omega.exp(fft_size_u128/m_u128);



// }