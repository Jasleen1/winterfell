use super::{rescue, Signature, CYCLE_LENGTH, NUM_HASH_ROUNDS, STATE_WIDTH};
use prover::math::field::{BaseElement, FieldElement};

// CONSTANTS
// ================================================================================================

const TWO: BaseElement = BaseElement::new(2);
const ZERO_KEY: [BaseElement; 2] = [BaseElement::ZERO, BaseElement::ZERO];

// TYPES AND INTERFACES
// ================================================================================================

struct KeySchedule {
    sec_keys1: Vec<[BaseElement; 2]>,
    sec_keys2: Vec<[BaseElement; 2]>,
    pub_keys1: Vec<[BaseElement; 2]>,
    pub_keys2: Vec<[BaseElement; 2]>,
}

// TRACE GENERATOR
// ================================================================================================

pub fn generate_trace(msg: &[BaseElement; 2], sig: &Signature) -> Vec<Vec<BaseElement>> {
    let m0 = msg[0].as_u128();
    let m1 = msg[1].as_u128();
    let key_schedule = build_key_schedule(m0, m1, sig);

    // initialize first state of the computation
    let mut state: [BaseElement; STATE_WIDTH] = [
        // message accumulators
        BaseElement::ONE,         // powers of two
        BaseElement::new(m0 & 1), // m0 bits
        BaseElement::new(m1 & 1), // m1 bits
        BaseElement::ZERO,        // m0 accumulator
        BaseElement::ZERO,        // m1 accumulator
        // secret key 1 hashing
        key_schedule.sec_keys1[0][0],
        key_schedule.sec_keys1[0][1],
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO, // capacity
        BaseElement::ZERO, // capacity
        // secret key 2 hashing
        key_schedule.sec_keys2[0][0],
        key_schedule.sec_keys2[0][1],
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO, // capacity
        BaseElement::ZERO, // capacity
        // public key hashing
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO, // capacity
        BaseElement::ZERO, // capacity
    ];

    // allocate memory to hold the trace table
    let trace_length = 128 * CYCLE_LENGTH;
    let mut trace = (0..STATE_WIDTH)
        .map(|_| vec![BaseElement::ZERO; trace_length])
        .collect::<Vec<Vec<BaseElement>>>();
    // copy initial state into the trace
    for (reg, &val) in state.iter().enumerate() {
        trace[reg][0] = val;
    }

    // execute the transition function for all steps
    for step in 0..(trace_length - 1) {
        // determine which cycle we are in and also where in the cycle we are
        let cycle_num = step / CYCLE_LENGTH;
        let cycle_pos = step % CYCLE_LENGTH;

        // break the state into logical parts
        let (mut msg_acc_state, rest) = state.split_at_mut(5);
        let (mut sec_key_1_hash, rest) = rest.split_at_mut(6);
        let (mut sec_key_2_hash, mut pub_key_hash) = rest.split_at_mut(6);

        if cycle_pos < NUM_HASH_ROUNDS {
            // for the first 7 steps in each cycle apply Rescue round function to
            // registers where keys are hashed; all other registers retain their values
            rescue::apply_round(&mut sec_key_1_hash, step);
            rescue::apply_round(&mut sec_key_2_hash, step);
            rescue::apply_round(&mut pub_key_hash, step);
        } else {
            let m0_bit = msg_acc_state[1];
            let m1_bit = msg_acc_state[2];

            // copy next set of public keys into the registers computing hash of the public key
            update_pub_key_hash(
                &mut pub_key_hash,
                m0_bit,
                m1_bit,
                &sec_key_1_hash,
                &sec_key_2_hash,
                &key_schedule.pub_keys1[cycle_num],
                &key_schedule.pub_keys2[cycle_num],
            );

            // copy next set of private keys into the registers computing private key hashes
            init_hash_state(&mut sec_key_1_hash, &key_schedule.sec_keys1[cycle_num + 1]);
            init_hash_state(&mut sec_key_2_hash, &key_schedule.sec_keys2[cycle_num + 1]);

            // update message accumulator with the next set of message bits
            apply_message_acc(&mut msg_acc_state, m0, m1, cycle_num);
        }

        // copy state into the trace
        for (reg, &val) in state.iter().enumerate() {
            trace[reg][step + 1] = val;
        }
    }

    trace
}

fn apply_message_acc(state: &mut [BaseElement], m0: u128, m1: u128, cycle_num: usize) {
    let power_of_two = state[0];
    let m0_bit = state[1];
    let m1_bit = state[2];

    state[0] = power_of_two * TWO;
    state[1] = BaseElement::from((m0 >> (cycle_num + 1)) & 1);
    state[2] = BaseElement::from((m1 >> (cycle_num + 1)) & 1);
    state[3] = state[3] + power_of_two * m0_bit;
    state[4] = state[4] + power_of_two * m1_bit;
}

fn init_hash_state(state: &mut [BaseElement], values: &[BaseElement; 2]) {
    state[0] = values[0];
    state[1] = values[1];
    state[2] = BaseElement::ZERO;
    state[3] = BaseElement::ZERO;
    state[4] = BaseElement::ZERO;
    state[5] = BaseElement::ZERO;
}

fn update_pub_key_hash(
    state: &mut [BaseElement],
    m0_bit: BaseElement,
    m1_bit: BaseElement,
    sec_key1_hash: &[BaseElement],
    sec_key2_hash: &[BaseElement],
    pub_key1: &[BaseElement],
    pub_key2: &[BaseElement],
) {
    if m0_bit == FieldElement::ONE {
        state[0] = state[0] + sec_key1_hash[0];
        state[1] = state[1] + sec_key1_hash[1];
    } else {
        state[0] = state[0] + pub_key1[0];
        state[1] = state[1] + pub_key1[1];
    }

    if m1_bit == FieldElement::ONE {
        state[2] = state[2] + sec_key2_hash[0];
        state[3] = state[3] + sec_key2_hash[1];
    } else {
        state[2] = state[2] + pub_key2[0];
        state[3] = state[3] + pub_key2[1];
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Transforms signature into 4 vectors of keys such that keys 0..127 and 127..254 end up in
/// different vectors; keys that are missing from the signature are replaced with a zeros.
fn build_key_schedule(m0: u128, m1: u128, sig: &Signature) -> KeySchedule {
    let mut n_ones = 0;
    let mut n_zeros = 0;
    let mut result = KeySchedule {
        sec_keys1: vec![ZERO_KEY; 128],
        sec_keys2: vec![ZERO_KEY; 128],
        pub_keys1: vec![ZERO_KEY; 128],
        pub_keys2: vec![ZERO_KEY; 128],
    };

    for i in 0..127 {
        if (m0 >> i) & 1 == 1 {
            result.sec_keys1[i] = sig.ones[n_ones];
            n_ones += 1;
        } else {
            result.pub_keys1[i] = sig.zeros[n_zeros];
            n_zeros += 1;
        }
    }

    for i in 0..127 {
        if (m1 >> i) & 1 == 1 {
            result.sec_keys2[i] = sig.ones[n_ones];
            n_ones += 1;
        } else {
            result.pub_keys2[i] = sig.zeros[n_zeros];
            n_zeros += 1;
        }
    }

    result
}
