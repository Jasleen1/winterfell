use prover::math::field::{BaseElement, FieldElement};

use super::{rescue, TreeNode, CYCLE_LENGTH, NUM_HASH_ROUNDS};

pub fn generate_trace(value: TreeNode, branch: &[TreeNode], index: usize) -> Vec<Vec<BaseElement>> {
    // allocate memory to hold the trace table
    let trace_length = branch.len() * CYCLE_LENGTH;
    let mut trace = vec![
        vec![BaseElement::ZERO; trace_length], // hash state
        vec![BaseElement::ZERO; trace_length], // hash state
        vec![BaseElement::ZERO; trace_length], // hash state
        vec![BaseElement::ZERO; trace_length], // hash state
        vec![BaseElement::ZERO; trace_length], // index bits
    ];

    let branch = &branch[1..];

    // initialize first state of the computation
    let mut state = [
        value.0,
        value.1,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
    ];
    // copy state into the trace
    for (reg, &val) in state.iter().enumerate() {
        trace[reg][0] = val;
    }

    // execute the transition function for all steps
    for step in 0..(trace_length - 1) {
        // determine which cycle we are in and also where in the cycle we are
        let cycle_num = step / CYCLE_LENGTH;
        let cycle_pos = step % CYCLE_LENGTH;

        // for the remaining 2 rounds, just carry over the values
        // in the first two registers to the next step
        if cycle_pos < NUM_HASH_ROUNDS {
            // in each of the first 14 steps, compute a single round of Rescue hash in
            // registers [0..3]
            rescue::apply_round(&mut state[..4], step);
            // the 5th register does not change during these steps
            state[4] = trace[4][step];
        } else if cycle_pos == NUM_HASH_ROUNDS {
            // on the 15th step, take the result of the hash from registers [0, 1],
            // and move it to the next step
            state[0] = trace[0][step];
            state[1] = trace[1][step];
            // set registers [2, 3] to 0
            state[2] = BaseElement::ZERO;
            state[3] = BaseElement::ZERO;
            // move next bit of the index into register 4
            state[4] = BaseElement::from(((index >> cycle_num) & 1) as u128);
        } else if cycle_pos == NUM_HASH_ROUNDS + 1 {
            // on the 16th step, copy next node of the branch into the appropriate position
            let index_bit = trace[4][step];
            if index_bit == BaseElement::ZERO {
                // if index bit is zero, accumulated hash goes into registers [0, 1],
                // and new branch node goes into registers [2, 3]
                state[0] = trace[0][step];
                state[1] = trace[1][step];
                state[2] = branch[cycle_num].0;
                state[3] = branch[cycle_num].1;
            } else {
                // if index bit is one, accumulated hash goes into registers [2, 3],
                // and new branch nodes goes into registers [0, 1]
                state[0] = branch[cycle_num].0;
                state[1] = branch[cycle_num].1;
                state[2] = trace[0][step];
                state[3] = trace[1][step];
            }
            // index bit is just copied over to the next step
            state[4] = trace[4][step];
        }

        // copy state into the trace
        for (reg, &val) in state.iter().enumerate() {
            trace[reg][step + 1] = val;
        }
    }

    trace
}
