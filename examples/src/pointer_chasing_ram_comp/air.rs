// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::{
    rescue::{self, STATE_WIDTH},
    BaseElement, ExtensionOf, FieldElement, ProofOptions, CYCLE_LENGTH, TRACE_WIDTH,
};
use crate::{
    pointer_chasing_ram_comp::usize_to_field,
    utils::{are_equal, not, EvaluationResult},
};
use core_utils::flatten_slice_elements;
use winterfell::{
    math::ToElements, Air, AirContext, Assertion, AuxTraceRandElements, EvaluationFrame, TraceInfo,
    TransitionConstraintDegree,
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
    //     pub inputs: [BaseElement; 2],
    pub result: BaseElement,
    pub num_locs: usize,
    pub num_steps: usize,
}

impl ToElements<BaseElement> for PublicInputs {
    fn to_elements(&self) -> Vec<BaseElement> {
        // let mut out = flatten_slice_elements(&[self.inputs]).to_vec();
        // out.push(self.result);
        // out
        let mut out = vec![self.result];
        out.push(usize_to_field(self.num_locs));
        out.push(usize_to_field(self.num_steps));
        out
    }
}

pub struct PointerChasingComponentAir {
    context: AirContext<BaseElement>,
    num_locs: usize,
    num_steps: usize,
    result: BaseElement,
}

impl Air for PointerChasingComponentAir {
    type BaseField = BaseElement;
    type PublicInputs = PublicInputs;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    fn new(trace_info: TraceInfo, pub_inputs: PublicInputs, options: ProofOptions) -> Self {
        let main_degrees = vec![TransitionConstraintDegree::with_cycles(1, vec![2]); 2];

        PointerChasingComponentAir {
            context: AirContext::new(trace_info, main_degrees, 2, options),
            num_locs: pub_inputs.num_locs,
            num_steps: pub_inputs.num_steps,
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

        // result.agg_constraint(
        //     0,
        //     periodic_values[0],
        //     are_equal(next[0], apply_next_loc_function(next[2])));

        result.agg_constraint(0, periodic_values[0], are_equal(next[2], current[1]));

        result.agg_constraint(
            1,
            E::ONE - periodic_values[0],
            are_equal(current[0], next[0]),
        );

        // result.agg_constraint(
        //     3,
        //     E::ONE - periodic_values[0],
        //     are_equal(current[0], apply_next_loc_function(current[2])));
    }

    fn get_assertions(&self) -> Vec<Assertion<Self::BaseField>> {
        // Assert starting and ending values of the hash chain
        let last_step = self.trace_length() - 1;
        vec![
            // Initial capacity registers must be set to zero
            Assertion::single(2, 0, usize_to_field(self.num_locs - 1)),
            // Final rate registers (digests) should be equal to
            // the provided public input
            Assertion::single(1, last_step, self.result),
        ]
    }

    fn get_periodic_column_values(&self) -> Vec<Vec<Self::BaseField>> {
        let mut result = vec![];
        let mut read_write_col = vec![BaseElement::ZERO, BaseElement::ONE];

        result.push(read_write_col);

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

fn apply_next_loc_function<E: FieldElement>(elt: E) -> E {
    elt
}
