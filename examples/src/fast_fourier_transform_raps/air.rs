// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use core::num;
use std::{convert::TryInto, thread::current, vec};

use super::{
    BaseElement, ExtensionOf, FieldElement, ProofOptions, prover::{get_num_cols, get_results_col_idx, get_num_steps},
};
use crate::utils::{are_equal, not, EvaluationResult};
use winterfell::{
    Air, AirContext, Assertion, AuxTraceRandElements, ByteWriter, EvaluationFrame, Serializable,
    TraceInfo, TransitionConstraintDegree, math::{StarkField, log2, fft},
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
        let num_fft_steps: usize = log2(pub_inputs.fft_inputs.len()).try_into().unwrap();
        let mut main_degrees = vec![TransitionConstraintDegree::with_cycles(1,vec![2]), TransitionConstraintDegree::with_cycles(1,vec![2])];
        for step in 2..num_fft_steps+1 {
            main_degrees.push(TransitionConstraintDegree::with_cycles(1, vec![2, 1<<step]));
            main_degrees.push(TransitionConstraintDegree::with_cycles(1, vec![2, 1<<step]));
        }
        main_degrees.push(TransitionConstraintDegree::new(1));
        // The constraints for the reverse perm columns
        main_degrees.push(TransitionConstraintDegree::new(1));
        main_degrees.push(TransitionConstraintDegree::new(1));
        let mut aux_degrees = vec![
            TransitionConstraintDegree::new(1), 
            TransitionConstraintDegree::new(1), 
            TransitionConstraintDegree::new(2),
        ];

        // for step in 0..num_fft_steps {
        //     let mut further_aux = vec![
        //         TransitionConstraintDegree::new(1), 
        //         TransitionConstraintDegree::new(1), 
        //        TransitionConstraintDegree::new(2),
        //     ];
        //     aux_degrees.append(&mut further_aux);
        // }

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
                2*pub_inputs.fft_inputs.len()+1,
                4*num_fft_steps - 2,//4,//pub_inputs.fft_inputs.len()-3,
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
        

        for step in 1..num_steps+1 {
            let local_omega = periodic_values[step-1];
            if step == 1 {
                let u = current[1];
                let v = next[1] * local_omega;

                result[2*step-2] = compute_flag * are_equal(u + v, current[2*step]);
                result[2*step-1] = compute_flag * are_equal(u - v, next[2*step]);
            }
            else {
                
                let u = current[3*(step-1)];
                let v = next[3*(step-1)] * local_omega;
            
                result[2*step-2] = compute_flag * are_equal(u + v, current[3*step - 2]);
                result[2*step-1] = compute_flag * are_equal(u - v, next[3*step - 2]);
            }
            

        }
        result[2*num_steps] = are_equal(current[last_col - 1] + E::ONE , next[last_col - 1]);

        self.evaluate_rev_perm(frame, &periodic_values.clone(), result, last_col);
        
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
        
        let main_current = main_frame.current();

        let aux_current = aux_frame.current();
        let aux_next = aux_frame.next();

        let random_elements = aux_rand_elements.get_segment_elements(0);
        
        let num_fft_inputs = self.fft_inputs.len();
        let fft_width = get_num_cols(num_fft_inputs);
        let num_steps = get_num_steps(num_fft_inputs);
        // // We want to enforce that the correct permutation was applied. Because we 
        // // want to copy two values per step: the actual value of the input and 
        // // its intended index, we group them with random elements into a single 
        // // cell via α_0 * c_0 + α_1 * c_1, where c_i is the appropriate permutation location.

        // // Note that storing the copied values into two auxiliary columns. One could
        // // instead directly compute the permutation argument, hence require a single
        // // auxiliary one. For the sake of illustrating RAPs behaviour, we will store
        // // the computed values in additional columns.
        let copied_value_1 = random_elements[0] * (main_current[0]).into()
            + random_elements[1] * (main_current[fft_width - 2]).into();

        result[0] = are_equal(aux_current[0], copied_value_1);

        let copied_value_2 = random_elements[0] * (main_current[1]).into()
            + random_elements[1] * (main_current[fft_width - 1]).into();

        result[1] = are_equal(aux_current[1], copied_value_2);
        
        // Enforce that the permutation argument column scales at each step by (aux[0] + γ) / (aux[1] + γ).
        result.agg_constraint(
            2,
            E::ONE,
            are_equal(
                aux_next[2] * (aux_current[1] + random_elements[2]),
                aux_current[2] * (aux_current[0] + random_elements[2]),
            ),
        );

        // println!("Periodic values: {:?}, {:?}, {:?}", periodic_values[2*num_steps + 1], 
                    // periodic_values[2*num_steps + 2], periodic_values[2*num_steps + 3]);
        // let new_loc = main_current[fft_width - 2] 
        //                 + (periodic_values[2*num_steps + 1])
        //                 - periodic_values[2*num_steps + 2] 
        //                 + periodic_values[2*num_steps + 3];

        // let copied_value_3 = random_elements[0] * (main_current[2]).into()
        //     + random_elements[1] * (main_current[fft_width - 2]).into();

        // result[3] = are_equal(aux_current[3], copied_value_3);

        // let copied_value_4 = random_elements[0] * (main_current[3]).into()
        //     + random_elements[1] * (new_loc).into();

        // result[4] = are_equal(aux_current[4], copied_value_4);
        
        // // Enforce that the permutation argument column scales at each step by (aux[0] + γ) / (aux[1] + γ).
        // result.agg_constraint(
        //     5,
        //     E::ONE,
        //     are_equal(
        //         aux_next[5] * (aux_current[4] + random_elements[2]),
        //         aux_current[5] * (aux_current[3] + random_elements[2]),
        //     ),
        // );

        // for step in 2..num_steps {
        //     let new_loc_forward_perm = main_current[fft_width - 2]
        //                                 + periodic_values[2*num_steps + 5 * (step-1) + 1]
        //                                 - periodic_values[2*num_steps + 5 * (step-1) + 2] 
        //                                 + periodic_values[2*num_steps + 5 * (step-1) + 3];
            
        //     let mut new_loc_backward_term = F::ZERO;
        //     let mut old_loc_backward_term = F::ZERO;
        //     if step >= 3 {  
        //         new_loc_backward_term = periodic_values[2*num_steps + 5 * (step - 1) + 4]
        //                                 - periodic_values[2*num_steps + 5 * (step - 1) + 5] 
        //                                 - (F::ONE - periodic_values[num_steps]);
        //         // old_loc_backward_perm = main_current[fft_width - 2];
        //     }
        //     println!("index = {:?}, step = {:?}", main_current[fft_width - 2], step);
            
        //     println!("periodics step - 1 = {:?}", 
        //             vec![periodic_values[2*num_steps + 5 * (step - 1) + 1], 
        //             periodic_values[2*num_steps + 5 * (step - 1) + 2], 
        //             periodic_values[2*num_steps + 5 * (step - 1) + 3]]);
        //     if step >= 3 {
        //         println!("Inv periodics step - 2 = {:?}", 
        //             vec![periodic_values[2*num_steps + 5 * (step - 2) + 4], 
        //             periodic_values[2*num_steps + 5 * (step - 2) + 5], 
        //             (F::ONE - periodic_values[num_steps])]);
        //     }

        //     let copy_a = random_elements[0] * main_current[2*step].into() 
        //                     + random_elements[1] * main_current[fft_width - 2].into();
        //                     //+ random_elements[3] * old_loc_backward_perm.into();
        //                     // random_elements[3] * new_loc_backward_perm.into();
        //     println!("Step = {:?}", step);
        //     println!("Original = {:?} New forward perm = {:?}",                      
        //             main_current[fft_width - 2],    
        //             new_loc_forward_perm 
        //             + new_loc_backward_term);
        //     // println!("Constraint # {}", 3*step);
        //     // println!("Aux current {:?}, copy_a {:?}", aux_current[3*step], copy_a);

        //     result[3*step] = are_equal(aux_current[3*step], copy_a);

        //     let copy_b = random_elements[0] * main_current[2*step+1].into() 
        //                     + random_elements[1] * (new_loc_forward_perm
        //                     +  new_loc_backward_term).into();

        //     result[3*step + 1] = are_equal(aux_current[3*step + 1], copy_b);

        // }
    }

    fn get_assertions(&self) -> Vec<Assertion<Self::BaseField>> {
        let num_cols = get_num_cols(self.fft_inputs.len());
        let results_col = get_results_col_idx(self.fft_inputs.len());
        
        // The last column should just keep a count of where you are.
        let mut assertions = vec![
            Assertion::single(num_cols - 2, 0, BaseElement::ZERO)
        ];
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
        let last_step = self.trace_length() - 1;
        let num_steps = get_num_steps(self.trace_length());
        let mut output_vec = vec![
            Assertion::single(2, 0, E::ONE),
            Assertion::single(2, last_step, E::ONE)
        ];
        for step in 2..num_steps+1 {
            output_vec.push(Assertion::single(3*step - 1, 0, E::ONE));
            output_vec.push(Assertion::single(3*step - 1, last_step, E::ONE));
        }
        for step in 2..num_steps+1 {
            output_vec.push(Assertion::single( 3*num_steps + 3*step - 4, 0, E::ONE));
            output_vec.push(Assertion::single(3*num_steps + 3*step - 4, last_step, E::ONE));
        }
        output_vec
        // vec![]
    }

    fn get_periodic_column_values(&self) -> Vec<Vec<Self::BaseField>> {
        let fft_size = self.fft_inputs.len();
        let fft_size_u128: u128 = fft_size.try_into().unwrap();
        let fft_size_u32: u32 = fft_size.try_into().unwrap();
        let num_steps: usize = log2(fft_size).try_into().unwrap();
        let mut result = Vec::<Vec::<BaseElement>>::new();
        let omega = BaseElement::get_root_of_unity(fft_size_u32);
        // We want to make sure we arrange it so that the appropriate omega can get multiplied.
        // Since the transition constraint must be identical at each step, 
        for step in 0..num_steps {
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

        // These flags are for indicating which step to compute on.
        let flags = vec![BaseElement::ONE,BaseElement::ZERO];
        result.push(flags);

        // These are periodic assertions to such that the next num_steps 
        // columns together represent the bit decomposition of the field elts 0-fft_size 
        let mut start_zeros = fft_size/2;
        for _ in 1..num_steps+1 {
            // For each bit in the indices of FFT inputs
            let mut bit_vec = vec![BaseElement::ZERO; start_zeros];
            let mut one_bit_vec = vec![BaseElement::ONE; start_zeros];
            bit_vec.append(&mut one_bit_vec);
            result.push(bit_vec);
            start_zeros = start_zeros / 2;
        }

        for j in 2..num_steps+1 {
            let jump = (1 << j)/2;
            let j_u64: u64 = j.try_into().unwrap();
            let jump_field_elt = BaseElement::new(2).exp(<BaseElement as FieldElement>::PositiveInteger::from(j_u64 - 1));
            
            let mut jump_col = vec![];
            let mut jump_col_first_half = vec![BaseElement::ZERO; jump];
            let mut jump_col_second_half = vec![jump_field_elt; jump];
            jump_col.append(&mut jump_col_first_half);
            jump_col.append(&mut jump_col_second_half);
            let mut parity_col_first_half = vec![BaseElement::ZERO; jump];
            let mut parity_col_second_half = vec![BaseElement::ONE; jump];
            let mut parity_col = vec![];
            parity_col.append(&mut parity_col_first_half);
            parity_col.append(&mut parity_col_second_half);

            let mut count = BaseElement::ZERO;
            let mut counter_col = vec![];
            let mut inv_counter_col = vec![];
            for _ in 0..jump {
                counter_col.push(count);
                // Append the count to the inv couter twice.
                // Adjacent elements in the inverse calculation have identical j's.
                inv_counter_col.push(count);
                inv_counter_col.push(count);
                // Increment the field element that keeps count
                count = count + BaseElement::ONE;
            }
            result.push(counter_col);
            result.push(jump_col);
            result.push(parity_col);


            result.push(vec![BaseElement::ZERO, jump_field_elt]);
            result.push(inv_counter_col);
        
        }

        // println!("Length of results vector {}", result.len());
        // println!("Next val in result {:?}", result[num_steps+1]);
        result
    }
}

impl FFTRapsAir {
    fn evaluate_rev_perm<E: FieldElement + From<<Self as Air>::BaseField>>(
        &self,
        frame: &EvaluationFrame<E>,
        periodic_values: &[E],
        result: &mut [E],
        last_col: usize,
    ) {
        let current = frame.current();
        let num_steps: usize = log2(self.fft_inputs.len()).try_into().unwrap();
        let mut backward_sum = E::ZERO;
        let mut forward_sum = E::ZERO;
        let mut backward_counter: u64 = 0;
        let mut forward_counter: u64 = num_steps.try_into().unwrap();
        for loc in num_steps+1..2*num_steps+1 {
            // Want to make sure we don't go below 0, 
            // so we subtract at the start of the loop iteration instead 
            // of starting at num_steps - 1.
            forward_counter = forward_counter - 1;
            let forward_pow = <E as FieldElement>::PositiveInteger::from(backward_counter);
            backward_sum = backward_sum + (periodic_values[loc] * E::from(2u128).exp(forward_pow));
            let backward_pow = <E as FieldElement>::PositiveInteger::from(forward_counter);
            forward_sum = forward_sum + (periodic_values[loc] * E::from(2u128).exp(backward_pow));
            backward_counter = backward_counter + 1;
        }
        result[2*num_steps + 1] = are_equal(forward_sum, current[last_col- 1]);
        result[2*num_steps + 2] = are_equal(backward_sum, current[last_col]);
    }

}





// HELPER EVALUATORS
// ------------------------------------------------------------------------------------------------


/*
x_i + perm(i)*gamma
y_i + i*gamma
*/

/*
X Y I J:=perm(i)
constraint: j = perm(i)
e: <E: FieldElement + From<E::BaseField>>
TODO:
- Is there a closed form (preferably algebraic) formula for checking the permutation at each step?
If so, then use that for a constraint.
*/