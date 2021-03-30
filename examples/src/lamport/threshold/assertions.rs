use crate::utils::bytes_to_node;

use super::{signature::AggPublicKey, HASH_CYCLE_LENGTH, SIG_CYCLE_LENGTH, STATE_WIDTH};
use math::field::{BaseElement, FieldElement};
use prover::Assertions;

// ASSERTION BUILDER
// ================================================================================================

#[rustfmt::skip]
pub fn build_assertions(
    pub_key: &AggPublicKey,
    message: [BaseElement; 2],
    num_signatures: usize,
) -> Assertions {
    let num_cycles = pub_key.num_keys().next_power_of_two();

    // create a collection to hold the assertions assertions
    let trace_length = SIG_CYCLE_LENGTH * num_cycles;
    let mut assertions = Assertions::new(STATE_WIDTH, trace_length).unwrap();

    // ----- assertions against the first step of every cycle: 0, 1024, 2048 etc. -----------------

    // message aggregators should be set to zeros
    assertions.add_cyclic(2, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(3, 0, num_cycles, BaseElement::ZERO).unwrap();
    // for private key hasher, last 4 state register should be set to zeros
    assertions.add_cyclic(6, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(7, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(8, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(9, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(12, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(13, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(14, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(15, 0, num_cycles, BaseElement::ZERO).unwrap();
    // for public key hasher, all registers should be set to zeros
    assertions.add_cyclic(16, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(17, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(18, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(19, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(20, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(21, 0, num_cycles, BaseElement::ZERO).unwrap();
    // for merkle path verification, last 4 registers should be set to zeros
    assertions.add_cyclic(24, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(25, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(26, 0, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(27, 0, num_cycles, BaseElement::ZERO).unwrap();
    // merkle path index accumulator should be initialized to zero
    assertions.add_cyclic(29, 0, num_cycles, BaseElement::ZERO).unwrap();

    // ----- assertions against the step in every cycle when the Merkle path computation ends -----
    // these steps depend on the depth of the public key Merkle tree; for example, if the Merkle 
    // tree has 4 elements, then the steps are: 24, 1048, 2072, 3096
    let merkle_root_offset = (num_cycles.trailing_zeros() + 1) as usize * HASH_CYCLE_LENGTH;

    // distinct key indexes should be used; the sequence starts at the last index of the tree
    // (to pad the first cycle) and then wraps around and proceeds with index 0, 1, 2 etc.
    let index_list = get_index_list(num_cycles);
    assertions.add_list(29, merkle_root_offset, index_list).unwrap();

    // merkle path verifications should terminate with the root public key
    let pub_key_root = bytes_to_node( pub_key.root());
    assertions.add_cyclic(22, merkle_root_offset, num_cycles, pub_key_root.0).unwrap();
    assertions.add_cyclic(23, merkle_root_offset, num_cycles, pub_key_root.1).unwrap();

    // ----- assertions against the last step of every cycle: 1023, 2047, 3071 etc. ----------------

    let last_cycle_step = SIG_CYCLE_LENGTH - 1;
    // last bits of message bit registers should be set to zeros; this is because we truncate
    // message elements to 127 bits each - so, 128th bit must always be zero
    assertions.add_cyclic(0, last_cycle_step, num_cycles, BaseElement::ZERO).unwrap();
    assertions.add_cyclic(1, last_cycle_step, num_cycles, BaseElement::ZERO).unwrap();
    // message accumulator registers should be set to message element values
    assertions.add_cyclic(2, last_cycle_step, num_cycles, message[0]).unwrap();
    assertions.add_cyclic(3, last_cycle_step, num_cycles, message[1]).unwrap();

    // ----- assertions for the entire execution trace --------------------------------------------
    // signature counter starts at zero and terminates with the expected count of signatures
    let last_step = trace_length - 1;
    assertions.add_single(31, 0, BaseElement::ZERO).unwrap();
    assertions.add_single(31, last_step, BaseElement::from(num_signatures as u64)).unwrap();
    
    // the first public key for merkle path verification should be a zero key (it is only used
    // for padding)
    assertions.add_single(22, 0, BaseElement::ZERO).unwrap();
    assertions.add_single(23, 0, BaseElement::ZERO).unwrap();

    assertions
}

// HELPER FUNCTIONS
// ================================================================================================
fn get_index_list(num_keys: usize) -> Vec<BaseElement> {
    let mut result = Vec::with_capacity(num_keys);
    result.push(BaseElement::from((num_keys - 1) as u64));
    for i in 0..(num_keys - 1) {
        result.push(BaseElement::from(i as u64));
    }
    result
}
