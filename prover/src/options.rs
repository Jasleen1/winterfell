use crypto::HashFunction;
use serde::{Deserialize, Serialize};

// TYPES AND INTERFACES
// ================================================================================================

// TODO: validate field values on de-serialization
#[derive(Clone, Serialize, Deserialize)]
pub struct ProofOptions {
    num_queries: u8,
    blowup_factor: u8, // stored as power of 2
    grinding_factor: u8,

    #[serde(with = "hash_fn_serialization")]
    hash_fn: HashFunction,
}

// PROOF OPTIONS IMPLEMENTATION
// ================================================================================================
impl ProofOptions {
    pub fn new(
        num_queries: usize,
        blowup_factor: usize,
        grinding_factor: u32,
        hash_fn: HashFunction,
    ) -> ProofOptions {
        assert!(num_queries > 0, "num_queries must be greater than 0");
        assert!(num_queries <= 128, "num_queries cannot be greater than 128");

        assert!(
            blowup_factor.is_power_of_two(),
            "blowup_factor must be a power of 2"
        );
        assert!(blowup_factor >= 4, "blowup_factor cannot be smaller than 4");
        assert!(
            blowup_factor <= 256,
            "blowup_factor cannot be greater than 256"
        );

        assert!(
            grinding_factor <= 32,
            "grinding factor cannot be greater than 32"
        );

        return ProofOptions {
            num_queries: num_queries as u8,
            blowup_factor: blowup_factor.trailing_zeros() as u8,
            grinding_factor: grinding_factor as u8,
            hash_fn,
        };
    }

    pub fn num_queries(&self) -> usize {
        return self.num_queries as usize;
    }

    pub fn blowup_factor(&self) -> usize {
        return 1 << (self.blowup_factor as usize);
    }

    pub fn grinding_factor(&self) -> u32 {
        return self.grinding_factor as u32;
    }

    pub fn hash_fn(&self) -> HashFunction {
        return self.hash_fn;
    }
}

// HASH FUNCTION SERIALIZATION / DE-SERIALIZATION
// ================================================================================================
mod hash_fn_serialization {

    use crypto::{hash, HashFunction};
    use serde::{de, ser, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(hf: &HashFunction, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *hf as usize {
            f if f == hash::blake3 as usize => s.serialize_u8(0),
            f if f == hash::sha3 as usize => s.serialize_u8(1),
            _ => Err(ser::Error::custom("unsupported hash function"))?,
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashFunction, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Deserialize::deserialize(deserializer)? {
            0u8 => Ok(hash::blake3),
            1u8 => Ok(hash::sha3),
            _ => Err(de::Error::custom("unsupported hash function")),
        }
    }
}
