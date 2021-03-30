use prover::{
    math::field::{BaseElement, FieldElement},
    ExecutionTrace,
};

use super::{rescue, CYCLE_LENGTH, NUM_HASH_ROUNDS};

pub fn generate_trace(seed: [BaseElement; 2], iterations: usize) -> ExecutionTrace {
    // allocate memory to hold the trace table
    let trace_length = iterations * CYCLE_LENGTH;
    let mut trace = ExecutionTrace::new(4, trace_length);

    trace.fill(
        |row| {
            // initialize first state of the computation
            row[0] = seed[0];
            row[1] = seed[1];
            row[2] = BaseElement::ZERO;
            row[3] = BaseElement::ZERO;
        },
        |step, state| {
            // execute the transition function for all steps
            //
            // for the first 14 steps in every cycle, compute a single round of
            // Rescue hash; for the remaining 2 rounds, just carry over the values
            // in the first two registers to the next step
            if (step % CYCLE_LENGTH) < NUM_HASH_ROUNDS {
                rescue::apply_round(state, step);
            } else {
                state[2] = BaseElement::ZERO;
                state[3] = BaseElement::ZERO;
            }
        },
    );

    trace
}
