use crypto::HashFunction;

pub struct FriOptions {
    folding_factor: usize,
    max_remainder_length: usize,
    blowup_factor: usize,
    hash_fn: HashFunction,
}

impl FriOptions {
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
}
