use super::{rescue, CYCLE_LENGTH, NUM_HASH_ROUNDS};
use prover::math::field::{FieldElement, StarkField};

pub fn generate_trace(seed: [FieldElement; 2], iterations: usize) -> Vec<Vec<FieldElement>> {
    // allocate memory to hold the trace table
    let trace_length = iterations * CYCLE_LENGTH;
    let mut trace = vec![
        vec![FieldElement::ZERO; trace_length],
        vec![FieldElement::ZERO; trace_length],
        vec![FieldElement::ZERO; trace_length],
        vec![FieldElement::ZERO; trace_length],
    ];

    // initialize first state of the computation
    let mut state = [seed[0], seed[1], FieldElement::ZERO, FieldElement::ZERO];
    // copy state into the trace
    for (reg, &val) in state.iter().enumerate() {
        trace[reg][0] = val;
    }

    // execute the transition function for all steps
    for step in 0..(trace_length - 1) {
        // for the first 14 steps in every cycle, compute a single round of
        // Rescue hash; for the remaining 2 rounds, just carry over the values
        // in the first two registers to the next step
        if (step % CYCLE_LENGTH) < NUM_HASH_ROUNDS {
            rescue::apply_round(&mut state, step);
        } else {
            state[0] = trace[0][step];
            state[1] = trace[1][step];
            state[2] = FieldElement::ZERO;
            state[3] = FieldElement::ZERO;
        }

        // copy state into the trace
        for (reg, &val) in state.iter().enumerate() {
            trace[reg][step + 1] = val;
        }
    }

    trace
}
