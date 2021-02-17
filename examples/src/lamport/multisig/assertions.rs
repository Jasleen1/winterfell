use super::{SIG_CYCLE_LENGTH, STATE_WIDTH};
use math::field::{BaseElement, FieldElement};
use prover::Assertions;

// ASSERTION BUILDER
// ================================================================================================

#[rustfmt::skip]
pub fn build_assertions(
    messages: &[[BaseElement; 2]],
    pub_keys: &[[BaseElement; 2]],
) -> Assertions {
    let num_cycles = messages.len();

    let messages = transpose(messages);
    let pub_keys = transpose(pub_keys);

    // create a collection to hold the assertions assertions
    let trace_length = SIG_CYCLE_LENGTH * num_cycles;
    let mut assertions = Assertions::new(STATE_WIDTH, trace_length).unwrap();

    // set assertions against the first step of every cycle: 0, 1024, 2048 etc.

    // message aggregators should be set to zeros
    assertions.add_cyclic_value(2, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(3, 0, num_cycles, BaseElement::ZERO).unwrap();
    // for private key hasher, last 4 state register should be set to zeros
    assertions.add_cyclic_value(6, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(7, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(8, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(9, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(12, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(13, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(14, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(15, 0, num_cycles, BaseElement::ZERO).unwrap();
    // for public key hasher, all registers should be set to zeros
    assertions.add_cyclic_value(16, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(17, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(18, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(19, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(20, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(21, 0, num_cycles, BaseElement::ZERO).unwrap();

    // set assertions against the last step of every cycle: 1023, 2047, 3071 etc.

    let last_cycle_step = SIG_CYCLE_LENGTH - 1;
    // last bits of message bit registers should be set to zeros; this is because we truncate
    // message elements to 127 bits each - so, 128th bit must always be zero
    assertions.add_cyclic_value(0, last_cycle_step, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic_value(1, last_cycle_step, num_cycles, BaseElement::ZERO).unwrap();
    // message accumulator registers should be set to message element values
    assertions.add_cyclic(2, last_cycle_step, messages.0).unwrap();
    assertions.add_cyclic(3, last_cycle_step, messages.1).unwrap();
    // public key hasher should terminate with public key elements
    assertions.add_cyclic(16, last_cycle_step, pub_keys.0).unwrap();
    assertions.add_cyclic(17, last_cycle_step, pub_keys.1).unwrap();
    assertions
}

// HELPER FUNCTIONS
// ================================================================================================
fn transpose(values: &[[BaseElement; 2]]) -> (Vec<BaseElement>, Vec<BaseElement>) {
    let n = values[0].len();
    let mut r1 = Vec::with_capacity(n);
    let mut r2 = Vec::with_capacity(n);
    for element in values {
        r1.push(element[0]);
        r2.push(element[1]);
    }
    (r1, r2)
}
