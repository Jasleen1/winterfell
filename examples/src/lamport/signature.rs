use prover::{
    crypto::hash::rescue_d,
    math::field::{BaseElement, FieldElement, StarkField},
};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

// CONSTANTS
// ================================================================================================

const MESSAGE_BITS: usize = 254;

// TYPES AND INTERFACES
// ================================================================================================

pub struct PrivateKey {
    seed: [u8; 32],
    sec_keys: Vec<[u8; 32]>,
    pub_keys: Vec<[u8; 32]>,
    pub_key_acc: [u8; 32],
}

impl PrivateKey {
    pub fn pub_key(&self) -> PublicKey {
        PublicKey(self.pub_key_acc)
    }
}

pub struct PublicKey([u8; 32]);

#[derive(Serialize, Deserialize)]
pub struct Signature {
    ones: Vec<[u8; 32]>,
    zeros: Vec<[u8; 32]>,
}

// PROCEDURES
// ================================================================================================

pub fn gen_keys(seed: [u8; 32]) -> PrivateKey {
    let keys = BaseElement::prng_vector(seed, MESSAGE_BITS * 2);
    let mut sec_keys = Vec::with_capacity(MESSAGE_BITS);
    let mut pub_keys = Vec::with_capacity(MESSAGE_BITS);
    let mut pub_key_acc = [0; 32];

    for i in (0..keys.len()).step_by(2) {
        let mut sk = [0; 32];
        sk[..16].copy_from_slice(&keys[i].to_bytes());
        sk[16..].copy_from_slice(&keys[i + 1].to_bytes());
        sec_keys.push(sk);

        let mut pk = [0; 32];
        rescue_d(&sk, &mut pk);

        let mut buf = [0; 64];
        buf[..32].copy_from_slice(&pk);
        buf[32..].copy_from_slice(&pub_key_acc);
        rescue_d(&buf, &mut pub_key_acc);

        pub_keys.push(pk);
    }
    PrivateKey {
        seed,
        sec_keys,
        pub_keys,
        pub_key_acc,
    }
}

pub fn sign(message: &[u8], key: &PrivateKey) -> Signature {
    let mut ones = Vec::new();
    let mut zeros = Vec::new();

    let mut n = 0;
    let chunks = to_chunks(message);
    for chunk in chunks.iter() {
        // make sure the least significant bit is 0
        assert_eq!(chunk & 1, 0);
        for i in 1..128 {
            if (chunk >> i) & 1 == 1 {
                ones.push(key.sec_keys[n]);
            } else {
                zeros.push(key.pub_keys[n]);
            }
            n += 1;
        }
    }

    Signature { ones, zeros }
}

pub fn verify(message: &[u8], pub_key: PublicKey, sig: &Signature) -> bool {
    let mut n_zeros = 0;
    let mut n_ones = 0;
    let mut pub_key_acc = [0; 32];
    let chunks = to_chunks(message);
    for chunk in chunks.iter() {
        // make sure the least significant bit is 0
        assert_eq!(chunk & 1, 0);
        for i in 1..128 {
            let mut buf = [0; 64];
            if (chunk >> i) & 1 == 1 {
                if n_ones == sig.ones.len() {
                    return false;
                }
                rescue_d(&sig.ones[n_ones], &mut buf[..32]);
                n_ones += 1;
            } else {
                if n_zeros == sig.zeros.len() {
                    return false;
                }
                buf[..32].copy_from_slice(&sig.zeros[n_zeros]);
                n_zeros += 1;
            }
            buf[32..].copy_from_slice(&pub_key_acc);
            rescue_d(&buf, &mut pub_key_acc);
        }
    }

    pub_key_acc == pub_key.0
}

// HELPER FUNCTIONS
// ================================================================================================

fn to_chunks(message: &[u8]) -> [u128; 2] {
    // reduce the message to a 32-byte value
    let hash = *blake3::hash(message).as_bytes();

    // interpret 32 bytes as two 128-bit integers
    let mut m0 = u128::from_le_bytes(hash[..16].try_into().unwrap());
    let mut m1 = (u128::from_le_bytes(hash[16..].try_into().unwrap()) >> 9) << 9;

    // clear the least significant bit of the first value to ensure that when parsed in big-endian
    // order, the value takes up at most 127 bits
    m0 = (m0 >> 1) << 1;

    // do the same thing with the second value, but also clear 8 more bits to make room for
    // checksum bits
    m1 = (m1 >> 9) << 9;

    // compute the checksum and put it into the least significant bits of the second values
    let checksum = m0.count_ones() + m1.count_ones();
    let m1 = m1 | (checksum << 1) as u128;

    [m0, m1]
}
