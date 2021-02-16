use super::{SIG_CYCLE_LENGTH, STATE_WIDTH};
use math::field::{BaseElement, FieldElement};
use prover::Assertions;

// ASSERTION BUILDER
// ================================================================================================

pub fn build_assertions(
    messages: &Vec<[BaseElement; 2]>,
    pub_keys: &Vec<[BaseElement; 2]>,
) -> Assertions {
    let num_signatures = messages.len();

    let messages = transpose(messages);
    let pub_keys = transpose(pub_keys);

    // create a collection to hold assertions
    let trace_length = SIG_CYCLE_LENGTH * num_signatures;
    let mut assertions = Assertions::new(STATE_WIDTH, trace_length).unwrap();

    // build a vector with zeros for each signature; this will be used to reset
    // trace states to zeros or ones when we start verifying a new signature
    let ones = vec![BaseElement::ONE; num_signatures];
    let zeros = vec![BaseElement::ZERO; num_signatures];

    // set assertions against the first step of every cycle: 0, 1024, 2048 etc.

    // power of two register is initialized to one
    assertions.add_cyclic(0, 0, ones.clone()).unwrap();
    // message aggregators are initialized to zeros
    assertions.add_cyclic(3, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(4, 0, zeros.clone()).unwrap();
    // last two rate registers and capacity registers are are initialized to zeros
    assertions.add_cyclic(7, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(8, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(9, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(10, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(13, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(14, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(15, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(16, 0, zeros.clone()).unwrap();
    // all public key registers are initialized to zeros
    assertions.add_cyclic(17, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(18, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(19, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(20, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(21, 0, zeros.clone()).unwrap();
    assertions.add_cyclic(22, 0, zeros.clone()).unwrap();

    // set assertions against the last step of every cycle: 1023, 2047, 3071 etc.

    let last_cycle_step = SIG_CYCLE_LENGTH - 1;
    // last bits of m0 and m1 are 0s
    assertions
        .add_cyclic(1, last_cycle_step, zeros.clone())
        .unwrap();
    assertions
        .add_cyclic(2, last_cycle_step, zeros.clone())
        .unwrap();
    // correct message was used during proof generation
    assertions
        .add_cyclic(3, last_cycle_step, messages.0)
        .unwrap();
    assertions
        .add_cyclic(4, last_cycle_step, messages.1)
        .unwrap();
    // correct public key was used during proof generation
    assertions
        .add_cyclic(17, last_cycle_step, pub_keys.0)
        .unwrap();
    assertions
        .add_cyclic(18, last_cycle_step, pub_keys.1)
        .unwrap();
    assertions
}

// HELPER FUNCTIONS
// ================================================================================================
fn transpose(values: &Vec<[BaseElement; 2]>) -> (Vec<BaseElement>, Vec<BaseElement>) {
    let n = values[0].len();
    let mut r1 = Vec::with_capacity(n);
    let mut r2 = Vec::with_capacity(n);
    for element in values {
        r1.push(element[0]);
        r2.push(element[1]);
    }
    (r1, r2)
}
