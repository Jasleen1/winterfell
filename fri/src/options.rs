use crypto::HashFunction;

// CONSTANTS
// ================================================================================================

// TODO: these are hard-coded for now, but in the future we should make them configurable
pub const MAX_REMAINDER_LENGTH: usize = 256;
pub const FOLDING_FACTOR: usize = 4;

// FRI OPTIONS
// ================================================================================================

#[derive(Clone)]
pub struct FriOptions {
    folding_factor: usize,
    max_remainder_length: usize,
    blowup_factor: usize,
    hash_fn: HashFunction,
}

impl FriOptions {
    pub fn new(blowup_factor: usize, hash_fn: HashFunction) -> Self {
        FriOptions {
            folding_factor: FOLDING_FACTOR,
            max_remainder_length: MAX_REMAINDER_LENGTH,
            blowup_factor,
            hash_fn,
        }
    }

    pub fn folding_factor(&self) -> usize {
        self.folding_factor
    }

    pub fn max_remainder_length(&self) -> usize {
        self.max_remainder_length
    }

    pub fn blowup_factor(&self) -> usize {
        self.blowup_factor
    }

    pub fn hash_fn(&self) -> HashFunction {
        self.hash_fn
    }

    pub fn num_fri_layers(&self, mut domain_size: usize) -> usize {
        let mut result = 0;
        while domain_size > self.max_remainder_length {
            domain_size /= self.folding_factor;
            result += 1;
        }
        result
    }

    pub fn fri_remainder_length(&self, mut domain_size: usize) -> usize {
        while domain_size > self.max_remainder_length {
            domain_size /= self.folding_factor;
        }
        domain_size
    }
}
