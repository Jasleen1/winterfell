// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::convert::TryInto;

use super::{
    prover::{get_num_cols, get_results_col_idx},
    BaseElement, ExtensionOf, FieldElement, ProofOptions,
};
use crate::utils::{are_equal, not, EvaluationResult};
use winterfell::{
    math::{fft, log2, StarkField},
    Air, AirContext, Assertion, AuxTraceRandElements, ByteWriter, EvaluationFrame, Serializable,
    TraceInfo, TransitionConstraintDegree,
};

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
        let num_fft_steps: usize = log2(pub_inputs.fft_inputs.len()).try_into().unwrap();
        let mut main_degrees = vec![
            TransitionConstraintDegree::with_cycles(1, vec![2]),
            TransitionConstraintDegree::with_cycles(1, vec![2]),
        ];
        for step in 2..num_fft_steps + 1 {
            main_degrees.push(TransitionConstraintDegree::with_cycles(
                1,
                vec![2, 1 << step],
            ));
            main_degrees.push(TransitionConstraintDegree::with_cycles(
                1,
                vec![2, 1 << step],
            ));
        }
        main_degrees.push(TransitionConstraintDegree::new(1));
        let aux_degrees = vec![];
        // let aux_degrees = vec![
        //     TransitionConstraintDegree::new(1);
        //     (pub_inputs.fft_inputs.len()-3)/2
        // ];
        // let log_num_inputs: usize = log2(pub_inputs.fft_inputs.len()).try_into().unwrap();
        // assert_eq!(2*log_num_inputs + 3, trace_info.width());
        // FFTAir {
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

        FFTAir {
            context: AirContext::new_multi_segment(
                trace_info,
                main_degrees,
                aux_degrees,
                2 * pub_inputs.fft_inputs.len() + 1,
                0, //pub_inputs.fft_inputs.len()-3,
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
        let last_col = get_num_cols(self.fft_inputs.len()) - 1;
        // You'll actually only check constraints at even steps, at odd steps you don't do anything
        let compute_flag = periodic_values[num_steps];
        for step in 1..num_steps + 1 {
            let local_omega = periodic_values[step - 1];
            let u = current[2 * step - 1];
            let v = next[2 * step - 1] * local_omega;

            result[2 * step - 2] = compute_flag * are_equal(u + v, current[2 * step]);
            result[2 * step - 1] = compute_flag * are_equal(u - v, next[2 * step]);
        }
        result[2 * num_steps] = are_equal(current[last_col] + E::ONE, next[last_col]);
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
        // let main_current = main_frame.current();
        // let main_next = main_frame.next();

        // let aux_current = aux_frame.current();
        // let aux_next = aux_frame.next();

        // let random_elements = aux_rand_elements.get_segment_elements(0);

        return;

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
        let num_cols = get_num_cols(self.fft_inputs.len());
        let results_col = get_results_col_idx(self.fft_inputs.len());

        // The last column should just keep a count of where you are.
        let mut assertions = vec![Assertion::single(num_cols - 1, 0, BaseElement::ZERO)];
        // The 0th column just includes fft inputs.
        for (row, &val) in self.fft_inputs.iter().enumerate() {
            assertions.push(Assertion::single(0, row, val))
        }
        // The second-to-last column is where the fft outputs are written down.
        for (row, &val) in self.result.iter().enumerate() {
            assertions.push(Assertion::single(results_col, row, val))
        }
        assertions
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
        let mut result = Vec::<Vec<BaseElement>>::new();
        let omega = BaseElement::get_root_of_unity(fft_size_u32);
        // println!("In the periodic col generation");
        for step in 0..num_steps {
            let m = 1 << (step + 1);
            let m_u128: u128 = m.try_into().unwrap();
            let mut local_omega_col = vec![BaseElement::ONE; m];
            let local_omega = omega.exp(fft_size_u128 / m_u128);
            for i in 0..m / 2 {
                let i_u128: u128 = i.try_into().unwrap();
                local_omega_col[2 * i] = local_omega.exp(i_u128);
            }
            // println!("Local omega col step {} = {:?}", step, local_omega_col);
            result.push(local_omega_col);
        }
        // println!("\n ******** \n");
        let flags = vec![BaseElement::ONE, BaseElement::ZERO];
        result.push(flags);
        result
    }
}

fn get_permuted_location_bit_rev<E: FieldElement + From<BaseElement>>(
    fft_size: usize,
    step: E,
) -> E {
    let step_base_elt = E::as_base_elements(&[step])[0];
    unimplemented!()
}

// HELPER EVALUATORS
// ------------------------------------------------------------------------------------------------
