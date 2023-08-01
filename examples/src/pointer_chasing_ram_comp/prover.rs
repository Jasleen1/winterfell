// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::num;

use winterfell::{math::log2, TraceTable};

use crate::utils::print_trace;

use super::{
    apply_rescue_round_parallel, rescue::STATE_WIDTH, usize_to_field, BaseElement,
    DefaultRandomCoin, ElementHasher, FieldElement, PhantomData, PointerChasingComponentAir,
    ProofOptions, Prover, PublicInputs, Trace, CYCLE_LENGTH, NUM_HASH_ROUNDS,
};

// RESCUE PROVER
// ================================================================================================
/// This example constructs a proof for correct execution of 2 hash chains simultaneously.
/// In order to demonstrate the power of RAPs, the two hash chains have seeds that are
/// permutations of each other.
pub struct PointerChasingComponentProver<H: ElementHasher> {
    options: ProofOptions,
    num_locs: usize,
    num_steps: usize,
    running_state: Vec<usize>,
    current_val: usize,
    _hasher: PhantomData<H>,
}

impl<H: ElementHasher> PointerChasingComponentProver<H> {
    pub fn new(options: ProofOptions, num_locs: usize, num_steps: usize) -> Self {
        let mut running_state = (0..num_locs).collect::<Vec<usize>>();
        Self {
            options,
            num_locs,
            num_steps,
            running_state,
            current_val: num_locs - 1,
            _hasher: PhantomData,
        }
    }
    /// The parameter `seeds` is the set of seeds for the first hash chain.
    /// The parameter `permuted_seeds` is the set of seeds for the second hash chain.
    pub fn build_trace(&mut self, input_1: usize, input_2: usize) -> TraceTable<BaseElement> {
        self.running_state[0] = self.running_state[0] + input_1;
        self.running_state[1] = self.running_state[1] + input_2;

        let log_num_locs: usize = log2(self.num_locs).try_into().unwrap();
        let mut trace = TraceTable::new(3 + log_num_locs + 1, 2 * self.num_steps);

        let init_val = self.current_val;
        let next_loc = self.get_next_loc(init_val);
        let next_val = self.running_state[next_loc];

        trace.fill(
            |state| {
                // initialize original chain
                state[0] = usize_to_field(next_loc);
                state[1] = usize_to_field(next_val);
                state[2] = usize_to_field(init_val);
                for i in 0..log_num_locs {
                    state[3 + i] = usize_to_field(((3 * init_val + 1) >> i) & 1);
                }
                state[3 + log_num_locs] = usize_to_field((3 * init_val + 1) >> log_num_locs);
            },
            |step, state| {
                // execute the transition function for all steps
                if (step + 1) % 2 == 1 {
                    // Write case
                    let loc = self.get_next_loc(self.current_val);
                    let prev_val = self.current_val;
                    let other_term = self.running_state[loc];
                    self.apply_plain_write_step(prev_val, loc);
                    let next_val = self.running_state[loc];

                    self.current_val = next_val;

                    state[1] = usize_to_field(next_val);
                    for i in 0..log_num_locs {
                        state[3 + i] = usize_to_field(((prev_val + other_term) >> i) & 1);
                    }
                    state[3 + log_num_locs] =
                        usize_to_field((prev_val + other_term) >> log_num_locs);
                } else {
                    let next_loc = self.get_next_loc(self.current_val);
                    let next_val = self.running_state[next_loc];

                    state[0] = usize_to_field(next_loc);
                    state[2] = state[1];
                    state[1] = usize_to_field(next_val);
                    for i in 0..log_num_locs {
                        state[3 + i] = usize_to_field(((3 * self.current_val + 1) >> i) & 1);
                    }
                    state[3 + log_num_locs] =
                        usize_to_field((3 * self.current_val + 1) >> log_num_locs);
                }
            },
        );
        // print_trace(&trace, 1, 0, 0..trace.width());
        trace
    }

    fn apply_plain_write_step(&mut self, previous_val: usize, read_loc: usize) {
        let next_val = self.combine_fn_plain(self.running_state[read_loc], previous_val);
        self.running_state[read_loc] = next_val;
    }

    fn get_next_loc(&mut self, val: usize) -> usize {
        (3 * val + 1) % self.num_locs
    }

    fn combine_fn_plain(&self, input_1: usize, input_2: usize) -> usize {
        (input_1 + input_2) % self.num_locs
    }
}

impl<H: ElementHasher> Prover for PointerChasingComponentProver<H>
where
    H: ElementHasher<BaseField = BaseElement>,
{
    type BaseField = BaseElement;
    type Air = PointerChasingComponentAir;
    type Trace = TraceTable<BaseElement>;
    type HashFn = H;
    type RandomCoin = DefaultRandomCoin<Self::HashFn>;

    fn get_pub_inputs(&self, trace: &Self::Trace) -> PublicInputs {
        let last_step = trace.length() - 1;
        // println!("Result from prover = {:?}", trace.get(1, last_step));
        PublicInputs {
            result: trace.get(1, last_step),
            num_locs: self.num_locs,
            num_steps: self.num_steps,
        }
    }

    fn options(&self) -> &ProofOptions {
        &self.options
    }
}
