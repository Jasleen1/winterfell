use digest::generic_array::ArrayLength;
use digest::Digest;
use std::convert::TryInto;
use std::marker::PhantomData;

/// Struct to store an extended private key.
/// TODO: reconsider Clone for private keys, just used it here to unblock.
#[derive(Clone)]
struct LamportPlusExtendedPrivateKey<D>
where
    D: Digest,
    D::OutputSize: ArrayLength<u8>,
{
    private_keys: Vec<[u8; 32]>,
    _marker: PhantomData<D>,
}

#[allow(dead_code)]
impl<D> LamportPlusExtendedPrivateKey<D>
where
    D: Digest,
    D::OutputSize: ArrayLength<u8>,
{
    /// Seed generation using as KDF a simple hash(seed || index).
    fn generate(seed: &[u8; 32]) -> Self {
        let hash_output_size = D::output_size() * 8;
        // TODO: update 10 to work dynamically for any hash output size
        let keys_size = hash_output_size + 10;
        let mut private_keys = Vec::with_capacity(keys_size);
        let mut hasher = D::new();

        for i in 0..keys_size {
            hasher.update(seed);
            hasher.update(i.to_le_bytes());
            private_keys.push(hasher.finalize_reset().as_slice().try_into().unwrap());
        }

        LamportPlusExtendedPrivateKey {
            private_keys,
            _marker: Default::default(),
        }
    }

    // TODO: consider signing hashes directly, thus remove this if the input is a message hash.
    /// Sign a message.
    fn sign(&self, message: &[u8]) -> LamportPlusSignature<D> {
        let extended_message = message_and_checksum::<D>(message);
        let length = extended_message.len();
        let mut signature: Vec<[u8; 32]> = Vec::with_capacity(message.len());
        let mut hasher = D::new();

        // Set message and checksum part.
        for (index, message_byte) in extended_message
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != length - 1)
        {
            for i in 0..8 {
                let key = if get_bit_at(message_byte, i) {
                    self.private_keys[index * 8 + i]
                } else {
                    hasher.update(self.private_keys[index * 8 + i]);
                    hasher.finalize_reset().as_slice().try_into().unwrap()
                };
                signature.push(key);
            }
        }

        // Final byte (zero_one_byte) where only two bits are required.
        // TODO extract method to avoid code duplication.
        for i in 0..2 {
            let key = if get_bit_at(&extended_message[length - 1], i) {
                self.private_keys[(length - 1) * 8 + i]
            } else {
                hasher.update(self.private_keys[(length - 1) * 8 + i]);
                hasher.finalize_reset().as_slice().try_into().unwrap()
            };
            signature.push(key);
        }

        LamportPlusSignature {
            signature,
            _marker: Default::default(),
        }
    }
}

/// Struct to store an extended public key.
#[derive(Clone)]
pub struct LamportPlusExtendedPublicKey<D>
where
    D: Digest,
    D::OutputSize: ArrayLength<u8>,
{
    public_keys: Vec<[u8; 32]>,
    _marker: PhantomData<D>,
}

impl<D> From<LamportPlusExtendedPrivateKey<D>> for LamportPlusExtendedPublicKey<D>
where
    D: Digest,
    D::OutputSize: ArrayLength<u8>,
{
    fn from(extended_private_key: LamportPlusExtendedPrivateKey<D>) -> Self {
        let mut hasher = D::new();
        let public_keys = extended_private_key
            .private_keys
            .iter()
            .map(|key| {
                hasher.update(key);
                hasher.finalize_reset().as_slice().try_into().unwrap()
            })
            .collect();

        LamportPlusExtendedPublicKey {
            public_keys,
            _marker: Default::default(),
        }
    }
}

/// Struct to store an extended public key.
#[derive(Clone)]
pub struct LamportPlusFinalPublicKey<D>
where
    D: Digest,
    D::OutputSize: ArrayLength<u8>,
{
    public_key: [u8; 32],
    _marker: PhantomData<D>,
}

impl<D> From<LamportPlusExtendedPublicKey<D>> for LamportPlusFinalPublicKey<D>
where
    D: Digest,
    D::OutputSize: ArrayLength<u8>,
{
    fn from(extended_public_key: LamportPlusExtendedPublicKey<D>) -> Self {
        let mut hasher = D::new();
        extended_public_key.public_keys.iter().for_each(|pub_key| {
            hasher.update(pub_key);
        });

        LamportPlusFinalPublicKey {
            public_key: hasher.finalize_reset().as_slice().try_into().unwrap(),
            _marker: Default::default(),
        }
    }
}

#[derive(Clone)]
/// Struct to store a signature output.
pub struct LamportPlusSignature<D>
where
    D: Digest,
    D::OutputSize: ArrayLength<u8>,
{
    signature: Vec<[u8; 32]>,
    _marker: PhantomData<D>,
}

#[allow(dead_code)]
impl<D> LamportPlusSignature<D>
where
    D: Digest,
    D::OutputSize: ArrayLength<u8>,
{
    /// Signature verification.
    fn verify(&self, message: &[u8], final_pub_key: LamportPlusFinalPublicKey<D>) -> bool {
        let extended_message = message_and_checksum::<D>(message);
        let length = extended_message.len();
        let mut extended_pub_key: Vec<[u8; 32]> = Vec::with_capacity(message.len());
        let mut hasher = D::new();

        // Set message and checksum part.
        for (index, message_byte) in extended_message
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != length - 1)
        {
            for i in 0..8 {
                let key = if get_bit_at(message_byte, i) {
                    hasher.update(self.signature[index * 8 + i]);
                    hasher.finalize_reset().as_slice().try_into().unwrap()
                } else {
                    self.signature[index * 8 + i]
                };
                extended_pub_key.push(key);
            }
        }

        // Final byte (zero_one_byte) where only two bits are required.
        for i in 0..2 {
            let key = if get_bit_at(&extended_message[length - 1], i) {
                hasher.update(self.signature[(length - 1) * 8 + i]);
                hasher.finalize_reset().as_slice().try_into().unwrap()
            } else {
                self.signature[(length - 1) * 8 + i]
            };
            extended_pub_key.push(key);
        }

        // Compute the extended public key.
        let extended_pub_key = LamportPlusExtendedPublicKey {
            public_keys: extended_pub_key,
            _marker: Default::default(),
        };

        // Compute the final "concatenated then hashed" public key.
        let computed_final_pub_key: LamportPlusFinalPublicKey<D> = extended_pub_key.into();

        // Compare input public key with the computed one.
        final_pub_key.public_key == computed_final_pub_key.public_key
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Return true if the bit is set at index.
fn get_bit_at(input_byte: &u8, index: usize) -> bool {
    input_byte & (1 << index) != 0
}

/// Return number of set bits in a Vec<u8>.
#[allow(clippy::ptr_arg)]
fn count_ones(input: &Vec<u8>) -> usize {
    let mut counter = 0;
    input.iter().for_each(|byte| counter += byte.count_ones());
    counter as usize
}

fn message_and_checksum<D>(message: &[u8]) -> Vec<u8>
where
    D: Digest,
    D::OutputSize: ArrayLength<u8>,
{
    let hash_output_size = D::output_size() * 8;
    let mut hasher = D::new();
    hasher.update(message);
    let mut message_hash = hasher.finalize_reset().as_slice().try_into().unwrap();

    // Count ones.
    let ones = count_ones(&message_hash);
    let use_ones = ones < hash_output_size / 2;

    // It doesn't matter if it's little or big endian because it's just one byte.
    // TODO note that we cannot sign the all zeros and all ones.
    //      We would need an extra key pair for that.
    let checksum_byte = if use_ones {
        (hash_output_size - ones).to_le_bytes()[0]
    } else {
        ones.to_le_bytes()[0]
    };

    // final two bits
    let zero_one_byte = if use_ones { 2u8 } else { 1u8 };

    // Just extend the message hash with the final two bytes.
    message_hash.push(checksum_byte);
    message_hash.push(zero_one_byte);
    message_hash
}

#[test]
fn lamport_plus_key_gen() {
    use blake3::Hasher as Blake3;

    let seed = [0u8; 32];
    let priv_key = LamportPlusExtendedPrivateKey::<Blake3>::generate(&seed);
    let pub_key: LamportPlusExtendedPublicKey<Blake3> = priv_key.clone().into();
    let final_pub_key: LamportPlusFinalPublicKey<Blake3> = pub_key.clone().into();

    assert_eq!(
        priv_key.private_keys.len(),
        Blake3::output_size() * 8 + 10,
        "Unexpected size of private keys"
    );
    assert_eq!(
        priv_key.private_keys.len(),
        pub_key.public_keys.len(),
        "Unexpected size of public keys"
    );

    let message = "Hello World 1".as_bytes();
    let sig = priv_key.sign(message);
    assert_eq!(
        priv_key.private_keys.len(),
        sig.signature.len(),
        "Unexpected size of signature"
    );
    assert!(sig.verify(message, final_pub_key.clone()));

    // Signature will fail for another message
    assert_eq!(
        sig.verify("Hello World 2".as_bytes(), final_pub_key.clone()),
        false
    );

    // Signature will fail for another public key
    let other_public_key = LamportPlusFinalPublicKey {
        public_key: [1u8; 32],
        _marker: Default::default(),
    };
    assert_eq!(sig.verify(message, other_public_key), false);

    // Sign and verify another message
    let message3 = "Hello World 3".as_bytes();
    let sig = priv_key.sign(message3);
    assert!(sig.verify(message3, final_pub_key.clone()));

    // Sign and verify the all zeros.
    // Although this passes, we shouldn't allow singing all zeros and all ones.
    let message_zeros = [0u8; 32];
    let sig = priv_key.sign(&message_zeros);
    assert!(sig.verify(&message_zeros, final_pub_key));

    // Helper prints
    // priv_key.private_keys.iter().for_each(| priv_key_part | println!("{:?}", priv_key_part));
    // pub_key.public_keys.iter().for_each(| pub_key_part | println!("{:?}", pub_key_part));
    // println!("{:?}", final_pub_key.public_key);
    // println!("{}", sig.verify(message, final_pub_key));
    // sig.signature.iter().for_each(| sig_part | println!("{:?}", sig_part));
}
