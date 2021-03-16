use super::{
    rescue, AggPublicKey, Signature, TreeNode, HASH_CYCLE_LENGTH, NUM_HASH_ROUNDS,
    SIG_CYCLE_LENGTH, STATE_WIDTH,
};
use prover::{
    math::field::{BaseElement, FieldElement, StarkField},
    ExecutionTrace,
};
use std::collections::HashMap;

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

pub fn generate_trace(
    pub_key: &AggPublicKey,
    message: [BaseElement; 2],
    signatures: &[(usize, Signature)],
) -> ExecutionTrace {
    // allocate memory to hold the trace table
    let num_cycles = pub_key.num_keys().next_power_of_two();
    let trace_length = SIG_CYCLE_LENGTH * num_cycles;
    let mut trace = (0..STATE_WIDTH)
        .map(|_| vec![BaseElement::ZERO; trace_length])
        .collect::<Vec<Vec<BaseElement>>>();

    // transform a list of signatures into a hashmap; this way we can look up signature
    // by index of the corresponding public key
    let mut signature_map = HashMap::new();
    for (i, sig) in signatures {
        signature_map.insert(i, sig);
    }

    // create a dummy signature; this will be used in place of signatures for keys
    // which did not sign the message
    let zero_sig = Signature {
        ones: vec![[BaseElement::ZERO; 2]; 254],
        zeros: vec![[BaseElement::ZERO; 2]; 254],
    };

    // iterate over all leaves of the aggregated public key; and if a signature exists for the
    // corresponding individual public key, use it go generate signature verification trace;
    // otherwise, use zero signature; for every non-zero signature, signature count is incremented
    let mut sig_count = 0;
    for i in 0..num_cycles {
        match signature_map.get(&i) {
            Some(sig) => {
                let sig_flag = BaseElement::from(1u64);
                append_sig_verification(
                    &mut trace, i, &message, &sig, sig_flag, pub_key, sig_count,
                );
                sig_count += 1;
            }
            None => {
                let sig_flag = BaseElement::from(0u64);
                append_sig_verification(
                    &mut trace, i, &message, &zero_sig, sig_flag, pub_key, sig_count,
                );
            }
        }
    }

    ExecutionTrace::init(trace)
}

fn append_sig_verification(
    trace: &mut Vec<Vec<BaseElement>>,
    index: usize,
    msg: &[BaseElement; 2],
    sig: &Signature,
    sig_flag: BaseElement,
    pub_key: &AggPublicKey,
    sig_count: usize,
) {
    let m0 = msg[0].as_int();
    let m1 = msg[1].as_int();
    let key_schedule = build_key_schedule(m0, m1, sig);

    // we verify that the individual public key exists in the aggregated public key after
    // we've verified the signature; thus, the key index is offset by 1. That is, when
    // we verify signature for pub key 1, we verify Merkle path for pub key 0; the last
    // verification wraps around, but we don't care since the last signature is always a
    // zero signature which does not affect the count.
    let key_index = sig_index_to_key_index(index, pub_key.num_leaves());
    let key_path = pub_key.get_leaf_path(key_index);
    let pub_key = pub_key.get_key(key_index).unwrap_or_default().to_elements();

    // initialize first state of signature verification
    let mut state: [BaseElement; STATE_WIDTH] = [
        // message accumulators
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
        // merkle path verification
        pub_key[0],
        pub_key[1],
        BaseElement::ZERO,
        BaseElement::ZERO,
        BaseElement::ZERO,                         // capacity
        BaseElement::ZERO,                         // capacity
        BaseElement::new((key_index & 1) as u128), // index bits
        BaseElement::ZERO,                         // index accumulator
        // signature counter
        sig_flag,                            // signature flag
        BaseElement::new(sig_count as u128), // signature count
    ];

    let first_step = index * SIG_CYCLE_LENGTH;
    let last_step = (index + 1) * SIG_CYCLE_LENGTH - 1;

    // copy initial state into the trace
    for (reg, &val) in state.iter().enumerate() {
        trace[reg][first_step] = val;
    }

    let mut power_of_two = BaseElement::ONE;

    // execute the transition function for all steps
    for step in first_step..last_step {
        // determine which cycle we are in and also where in the cycle we are
        let cycle_num = (step % SIG_CYCLE_LENGTH) / HASH_CYCLE_LENGTH;
        let cycle_step = (step % SIG_CYCLE_LENGTH) % HASH_CYCLE_LENGTH;

        // break the state into logical parts; we don't need to do anything with sig_count part
        // because values for these registers are set in the initial state and don't change
        // during the cycle
        let (mut msg_acc_state, rest) = state.split_at_mut(4);
        let (mut sec_key_1_hash, rest) = rest.split_at_mut(6);
        let (mut sec_key_2_hash, rest) = rest.split_at_mut(6);
        let (mut pub_key_hash, rest) = rest.split_at_mut(6);
        let (mut merkle_path_hash, rest) = rest.split_at_mut(6);
        let (mut merkle_path_idx, _sig_count) = rest.split_at_mut(2);

        if cycle_step < NUM_HASH_ROUNDS {
            // for the first 7 steps in each hash cycle apply Rescue round function to
            // registers where keys are hashed; all other registers retain their values
            rescue::apply_round(&mut sec_key_1_hash, cycle_step);
            rescue::apply_round(&mut sec_key_2_hash, cycle_step);
            rescue::apply_round(&mut pub_key_hash, cycle_step);
            rescue::apply_round(&mut merkle_path_hash, cycle_step);
        } else {
            // for the 8th step of very cycle do the following:

            let m0_bit = msg_acc_state[0];
            let m1_bit = msg_acc_state[1];
            let mp_bit = merkle_path_idx[0];

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
            apply_message_acc(&mut msg_acc_state, m0, m1, cycle_num, power_of_two);

            // update merkle path index accumulator with the next index bit
            update_merkle_path_index(
                &mut merkle_path_idx,
                key_index as u128,
                cycle_num,
                power_of_two,
            );
            // prepare Merkle path hashing registers for hashing of the next node
            update_merkle_path_hash(&mut merkle_path_hash, mp_bit, cycle_num, &key_path);

            power_of_two = power_of_two * TWO;
        }

        // copy state into the trace
        for (reg, &val) in state.iter().enumerate() {
            trace[reg][step + 1] = val;
        }
    }
}

fn apply_message_acc(
    state: &mut [BaseElement],
    m0: u128,
    m1: u128,
    cycle_num: usize,
    power_of_two: BaseElement,
) {
    let m0_bit = state[0];
    let m1_bit = state[1];

    state[0] = BaseElement::from((m0 >> (cycle_num + 1)) & 1);
    state[1] = BaseElement::from((m1 >> (cycle_num + 1)) & 1);
    state[2] = state[2] + power_of_two * m0_bit;
    state[3] = state[3] + power_of_two * m1_bit;
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

fn update_merkle_path_hash(
    state: &mut [BaseElement],
    index_bit: BaseElement,
    cycle_num: usize,
    key_path: &[TreeNode],
) {
    let h1 = state[0];
    let h2 = state[1];
    let cycle_num = (cycle_num + 1) % key_path.len();
    if index_bit == BaseElement::ONE {
        state[0] = key_path[cycle_num].0;
        state[1] = key_path[cycle_num].1;
        state[2] = h1;
        state[3] = h2;
    } else {
        state[0] = h1;
        state[1] = h2;
        state[2] = key_path[cycle_num].0;
        state[3] = key_path[cycle_num].1;
    }
    state[4] = BaseElement::ZERO;
    state[5] = BaseElement::ZERO;
}

fn update_merkle_path_index(
    state: &mut [BaseElement],
    index: u128,
    cycle_num: usize,
    power_of_two: BaseElement,
) {
    let index_bit = state[0];
    // the cycle is offset by +1 because the first node in the Merkle path is redundant and we
    // get it by hashing the public key
    state[0] = BaseElement::from((index >> (cycle_num + 1)) & 1);
    state[1] = state[1] + power_of_two * index_bit;
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

fn sig_index_to_key_index(sig_index: usize, num_cycles: usize) -> usize {
    if sig_index == 0 {
        num_cycles - 1
    } else {
        sig_index - 1
    }
}
