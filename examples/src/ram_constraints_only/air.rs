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
            vec![TransitionConstraintDegree::new(2); log_steps_usize + 1];
        transition_constraint_degrees.push(TransitionConstraintDegree::new(2));
        transition_constraint_degrees.push(TransitionConstraintDegree::new(2));
        transition_constraint_degrees.push(TransitionConstraintDegree::new(5));
        transition_constraint_degrees.push(TransitionConstraintDegree::new(1));

        transition_constraint_degrees.push(TransitionConstraintDegree::new(4 * log_steps_usize));
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
        _periodic_values: &[E],
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
        for i in 4..(4 + log_steps_usize) {
            result.agg_constraint(
                i - 4,
                E::ONE,
                are_equal(current[i] * current[i], current[i]),
            );
        }

        // Check that op is also bits
        result.agg_constraint(
            log_steps_usize,
            E::ONE,
            are_equal(next[1] * next[1], next[1]),
        );

        // Check that you can correctly compute the function
        // f(loc_i, loc_{i+1}) = {1 if  they are equal, 0 otherwise}
        result.agg_constraint(
            log_steps_usize + 1,
            E::ONE,
            are_equal(
                current[4 + log_steps_usize] * current[4 + log_steps_usize + 1],
                E::ONE,
            ),
        );

        // Check that you can correctly compute the function
        // f(val_i, val_{i+1}) = {1 if  they are equal, 0 otherwise}
        result.agg_constraint(
            log_steps_usize + 2,
            E::ONE,
            are_equal(
                current[4 + log_steps_usize + 2] * current[4 + log_steps_usize + 3],
                E::ONE,
            ),
        );

        // Check that at any step (loc_i = loc_{i+1}) implies (op_{i+1} = write) OR (val_i = val_{i + 1})
        // this is equivalent to checking not(loc_i = loc_{i+1}) OR (op_{i+1} = write) OR (val_i = val_{i + 1})
        result.agg_constraint(
            log_steps_usize + 3,
            E::ONE,
            are_equal(
                compute_or(
                    compute_or(
                        E::ONE.sub(compute_f(current[2], next[2], next[4 + log_steps_usize])),
                        E::ONE.sub(are_equal(next[1], E::ONE)),
                    ),
                    compute_f(current[3], next[3], next[4 + log_steps_usize + 2]),
                ),
                E::ONE,
            ),
        );
        //
        // Check that we have the correct decomposition of the RAM step indices
        result.agg_constraint(
            log_steps_usize + 4,
            E::ONE,
            are_equal(
                current[0],
                compute_from_bit_decomp(&current[4..4 + log_steps_usize]),
            ),
        );

        // Check that [(l_i = l_{i+1}) AND (t_{i} < t_{i + 1})] OR [l_{i+1} = l_i + 1]
        // This reduces to checking (compute_f(l_i, l_{i+1}, .) * greater_than(t_next, t_curr)) +
        //  (1 - compute_f(l_i, l_{i+1}, .) * (l_{i+1} - l_i))
        let f_val = compute_f(current[2], next[2], next[4 + log_steps_usize]);
        result.agg_constraint(
            log_steps_usize + 5,
            E::ONE,
            are_equal(
                E::ONE,
                (f_val.mul(bit_vec_gt(
                    &next[4..4 + log_steps_usize],
                    &current[4..4 + log_steps_usize],
                )))
                .add((E::ONE.sub(f_val)).mul(next[2].sub(current[2]))),
            ),
        );
    }

    fn get_assertions(&self) -> Vec<Assertion<Self::BaseField>> {
        // Assert starting and ending values of the hash chain
        // let last_step = self.trace_length() - 1;
        vec![Assertion::single(2, 0, BaseElement::ZERO)]
    }

    fn get_periodic_column_values(&self) -> Vec<Vec<Self::BaseField>> {
        let result = vec![];
        // let mut absorption_column = vec![BaseElement::ZERO; CYCLE_LENGTH];
        // absorption_column[14] = BaseElement::ONE;
        // result.push(absorption_column);

        // result.append(&mut rescue::get_round_constants());

        result
    }
}

// HELPER EVALUATORS
// ------------------------------------------------------------------------------------------------

fn compute_or<E: FieldElement>(a: E, b: E) -> E {
    a.add(b).sub(a.mul(b))
}

fn compute_f<E: FieldElement>(a: E, b: E, c: E) -> E {
    E::ONE.sub(c.mul(a.sub(b)))
}

fn compute_from_bit_decomp<E: FieldElement>(decompositon: &[E]) -> E {
    let mut sum = E::ZERO;
    for (pow, &elt) in decompositon.iter().enumerate() {
        sum = sum.add(elt.mul(E::from(1u64 << pow)));
    }
    sum
}

fn bit_greater_than<E: FieldElement>(a: E, b: E) -> E {
    // outputs 1 if a > b and 0 otherwise. Must check that a and b are bits elsewhere
    // degree 2
    a.mul(E::ONE.sub(b))
}

fn bit_equals<E: FieldElement>(a: E, b: E) -> E {
    // outputs 1 if a = b and 0 otherwise. Must check that a and b are bits elsewhere
    // degree 2
    (a.mul(b)).add((E::ONE.sub(a)).mul(E::ONE.sub(b)))
}

fn bit_vec_gt<E: FieldElement>(a: &[E], b: &[E]) -> E {
    // outputs 1 if the integer computed using [`compute_from_bit_decomp`]
    // from a is greater than the int computed the same way from b.
    // assumes a and b are both the same length!
    // Also assumes that a and b are bot non-empty.
    let n = a.len();
    if a.len() == 1 {
        bit_greater_than(a[n - 1], b[n - 1])
    } else {
        // if a[n-1]>b[n-1], we'll return 1, if a[n-1] == b[n-1], we'll recurse
        compute_or(
            bit_greater_than(a[n - 1], b[n - 1]),
            bit_equals(b[n - 1], a[n - 1]).mul(bit_vec_gt(&a[0..n - 1], &b[0..n - 1])),
        )
    }
}
