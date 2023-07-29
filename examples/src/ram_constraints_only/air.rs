// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::{
    rescue::{self, STATE_WIDTH},
    BaseElement, ExtensionOf, FieldElement, ProofOptions,
};
use crate::utils::{are_equal, not, EvaluationResult};
use core_utils::flatten_slice_elements;
use winterfell::{
    math::{log2, ToElements},
    Air, AirContext, Assertion, AuxTraceRandElements, EvaluationFrame, TraceInfo,
    TransitionConstraintDegree,
};

// // CONSTANTS
// // ================================================================================================

// /// Specifies steps on which Rescue transition function is applied.
// const CYCLE_MASK: [BaseElement; CYCLE_LENGTH] = [
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ONE,
//     BaseElement::ZERO,
//     BaseElement::ZERO,
// ];

// RESCUE AIR
// ================================================================================================

pub struct PublicInputs {
    pub num_locs: u64,
    pub num_ram_steps: u64,
}

impl ToElements<BaseElement> for PublicInputs {
    fn to_elements(&self) -> Vec<BaseElement> {
        vec![
            BaseElement::from(self.num_locs),
            BaseElement::from(self.num_ram_steps),
        ]
    }
}

pub struct RamConstraintsAir {
    context: AirContext<BaseElement>,
    public_inputs: PublicInputs,
}

impl Air for RamConstraintsAir {
    type BaseField = BaseElement;
    type PublicInputs = PublicInputs;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(trace_info: TraceInfo, public_inputs: PublicInputs, options: ProofOptions) -> Self {
        // let main_degrees =
        //     vec![TransitionConstraintDegree::with_cycles(3, vec![CYCLE_LENGTH]); 2 * STATE_WIDTH];
        let log_locs_usize: usize = log2(public_inputs.num_locs.try_into().unwrap())
            .try_into()
            .unwrap();
        let log_steps_usize: usize = log2(public_inputs.num_ram_steps.try_into().unwrap())
            .try_into()
            .unwrap();
        let mut transition_constraint_degrees =
            vec![TransitionConstraintDegree::new(2); log_locs_usize + log_steps_usize + 1];
        transition_constraint_degrees.push(TransitionConstraintDegree::new(2));
        transition_constraint_degrees.push(TransitionConstraintDegree::new(2));
        // let aux_degrees = vec![];
        // assert_eq!(TRACE_WIDTH + 3, trace_info.width());
        RamConstraintsAir {
            context: AirContext::new(trace_info, transition_constraint_degrees, 1, options),
            public_inputs,
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

        let log_locs_usize: usize = log2(self.public_inputs.num_locs.try_into().unwrap())
            .try_into()
            .unwrap();
        let log_steps_usize: usize = log2(self.public_inputs.num_ram_steps.try_into().unwrap())
            .try_into()
            .unwrap();
        for i in 4..(4 + log_locs_usize + log_steps_usize) {
            result.agg_constraint(
                i - 4,
                E::ONE,
                are_equal(current[i] * current[i], current[i]),
            );
        }

        // Check that op is also bits
        result.agg_constraint(
            log_locs_usize + log_steps_usize,
            E::ONE,
            are_equal(current[1] * current[1], current[1]),
        );
        
        // Check that you can correctly compute the function 
        // f(loc_i, loc_{i+1}) = {1 if  they are equal, 0 otherwise}
        result.agg_constraint(
            log_locs_usize + log_steps_usize + 1, 
            E::ONE, 
            are_equal(current[4 + log_locs_usize + log_steps_usize] * current[4 + log_locs_usize + log_steps_usize + 1], E::ONE)
        );

        // Check that you can correctly compute the function 
        // f(val_i, val_{i+1}) = {1 if  they are equal, 0 otherwise}
        result.agg_constraint(
            log_locs_usize + log_steps_usize + 2, 
            E::ONE, 
            are_equal(current[4 + log_locs_usize + log_steps_usize + 2] * current[4 + log_locs_usize + log_steps_usize + 3], E::ONE)
        );
        
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

        // let absorption_flag = periodic_values[1];

        // // We want to enforce that the absorbed values of the first hash chain are a
        // // permutation of the absorbed values of the second one. Recall that the type
        // // for both seed and permuted_seed (the arrays being hashed into the chain), was
        // // [[BaseElement; 2]] and we never permute any of the internal arrays, since
        // // each [BaseElement; 2] represents the capacity registers for a single link in the
        // // hash chain. Due to this, we want to copy two values per hash chain at iteration
        // // (namely, the two capacity registers). To reduce the number of auxiliary registers needed
        // // to represent each link, we group them with random elements into a single cell via
        // // α_0 * c_0 + α_1 * c_1, where c_i is computed as next_i - current_i.

        // // Note that the reason we use next_i - current_i is that we are
        // // absorbing the new seed by adding it to the output of the previous hash.

        // // Note that storing the copied values into two auxiliary columns. One could
        // // instead directly compute the permutation argument, hence require a single
        // // auxiliary one. For the sake of illustrating RAPs behaviour, we will store
        // // the computed values in additional columns.

        // let copied_value_1 = random_elements[0] * (main_next[0] - main_current[0]).into()
        //     + random_elements[1] * (main_next[1] - main_current[1]).into();

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
        let last_step = self.trace_length() - 1;
        vec![Assertion::single(2, 0, BaseElement::ZERO)]
    }

    fn get_aux_assertions<E: FieldElement + From<Self::BaseField>>(
        &self,
        _aux_rand_elements: &AuxTraceRandElements<E>,
    ) -> Vec<Assertion<E>> {
        let last_step = self.trace_length() - 1;
        vec![]
    }

    fn get_periodic_column_values(&self) -> Vec<Vec<Self::BaseField>> {
        let mut result = vec![];
        // let mut absorption_column = vec![BaseElement::ZERO; CYCLE_LENGTH];
        // absorption_column[14] = BaseElement::ONE;
        // result.push(absorption_column);

        // result.append(&mut rescue::get_round_constants());

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

// fn enforce_bits<E: FieldElement>(result: &mut [E], current: &[E], next: &[E], flag: E) {
//     result.(0, flag, are_equal(current[0], next[0]));

// }
