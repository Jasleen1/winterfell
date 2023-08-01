// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use winterfell::{math::log2, TraceTable};

use crate::utils::compute_equality_cols;

use super::{
    BaseElement, DefaultRandomCoin, ElementHasher, FieldElement, PhantomData, ProofOptions, Prover,
    PublicInputs, RamConstraintsAir, Trace,
};

// RESCUE PROVER
// ================================================================================================
/// This example constructs a proof for correct execution of 2 hash chains simultaneously.
/// In order to demonstrate the power of RAPs, the two hash chains have seeds that are
/// permutations of each other.
pub struct RamConstraintProver<H: ElementHasher> {
    options: ProofOptions,
    num_locs: usize,
    num_ram_steps: usize,
    _hasher: PhantomData<H>,
}

impl<H: ElementHasher> RamConstraintProver<H> {
    pub fn new(options: ProofOptions, num_locs: usize, num_ram_steps: usize) -> Self {
        Self {
            options,
            num_locs,
            num_ram_steps,
            _hasher: PhantomData,
        }
    }

    pub fn build_trace(&self, valid_ram: &Vec<[u64; 4]>) -> TraceTable<BaseElement> {
        // allocate memory to hold the trace table
        let trace_length = self.num_ram_steps;
        let log_ram_steps_usize: usize = log2(self.num_ram_steps).try_into().unwrap();
        let log_ram_locs_usize: usize = log2(self.num_locs).try_into().unwrap();
        let trace_width = 4 + log_ram_steps_usize + 4;
        let mut trace = TraceTable::new(trace_width, trace_length);

        trace.fill(
            |state| {
                // initialize the original values
                state[0] = BaseElement::from(valid_ram[0][0]);
                state[1] = BaseElement::from(99u64);
                state[2] = BaseElement::from(valid_ram[0][2]);
                state[3] = BaseElement::from(valid_ram[0][3]);

                // // initialize bit decomp of location
                // for i in 0..log_ram_locs_usize {
                //     let ith_bit = (valid_ram[0][2] >> i) & 1;
                //     state[4 + i] = BaseElement::from(ith_bit);
                // }
                // initialize the bit decomposition of the size
                for i in 0..log_ram_steps_usize {
                    let ith_bit = (valid_ram[0][0] >> i) & 1;
                    state[4 + i] = BaseElement::from(ith_bit);
                }
                // These are just throw-aways in the initial step
                let loc_equality_terms =
                    compute_equality_cols(BaseElement::from(valid_ram[0][2]));
                state[4 + log_ram_steps_usize] = loc_equality_terms[0];
                state[4 + log_ram_steps_usize + 1] = loc_equality_terms[1];

                let val_equality_terms =
                    compute_equality_cols(BaseElement::from(valid_ram[0][3]));
                state[4 + log_ram_steps_usize + 2] = val_equality_terms[0];
                state[4 + log_ram_steps_usize + 3] = val_equality_terms[1];
            },
            |step, state| {
                // execute the transition function for all steps
                // initialize the original values
                // t := timestep
                state[0] = BaseElement::from(valid_ram[step + 1][0]);
                // op_t
                state[1] = BaseElement::from(valid_ram[step + 1][1]);
                // loc_t
                state[2] = BaseElement::from(valid_ram[step + 1][2]);
                // val_t
                state[3] = BaseElement::from(valid_ram[step + 1][3]);

                // // initialize bit decomp of location
                // for i in 0..log_ram_locs_usize {
                //     let ith_bit = (valid_ram[step + 1][2] >> i) & 1;
                //     state[4 + i] = BaseElement::from(ith_bit);
                // }
                // initialize the bit decomposition of the size
                for i in 0..log_ram_steps_usize {
                    let ith_bit = (valid_ram[step + 1][0] >> i) & 1;
                    state[4 + i] = BaseElement::from(ith_bit);
                }

                let loc_equality_terms = compute_equality_cols(
                    BaseElement::from(valid_ram[step][2])
                        - BaseElement::from(valid_ram[step + 1][2]),
                );

                state[4 + log_ram_steps_usize] = loc_equality_terms[0];
                state[4 + log_ram_steps_usize + 1] = loc_equality_terms[1];

                let val_equality_terms = compute_equality_cols(
                    BaseElement::from(valid_ram[step][3])
                        - BaseElement::from(valid_ram[step + 1][3]),
                );
                state[4 + log_ram_steps_usize + 2] = val_equality_terms[0];
                state[4 + log_ram_steps_usize + 3] = val_equality_terms[1];
            },
        );

        // debug_assert_eq!(trace.get(0, trace_length - 1), result[0][0]);
        // debug_assert_eq!(trace.get(1, trace_length - 1), result[0][1]);

        // debug_assert_eq!(trace.get(4, trace_length - 1), result[1][0]);
        // debug_assert_eq!(trace.get(5, trace_length - 1), result[1][1]);

        trace
    }

    
}



impl<H: ElementHasher> Prover for RamConstraintProver<H>
where
    H: ElementHasher<BaseField = BaseElement>,
{
    type BaseField = BaseElement;
    type Air = RamConstraintsAir;
    type Trace = TraceTable<BaseElement>;
    type HashFn = H;
    type RandomCoin = DefaultRandomCoin<Self::HashFn>;

    fn get_pub_inputs(&self, trace: &Self::Trace) -> PublicInputs {
        PublicInputs {
            num_locs: self.num_locs.try_into().unwrap(),
            num_ram_steps: self.num_ram_steps.try_into().unwrap(),
        }
    }

    fn options(&self) -> &ProofOptions {
        &self.options
    }
}
