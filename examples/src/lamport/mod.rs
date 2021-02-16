use super::Example;

pub mod multisig;
pub mod single;

mod signature;
use signature::{message_to_elements, PrivateKey, Signature};

mod rescue;
use rescue::{CYCLE_LENGTH, NUM_ROUNDS as NUM_HASH_ROUNDS};

#[cfg(test)]
mod tests;
