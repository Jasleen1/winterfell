use crate::HashFunction;
use math::field::FieldElement;
use std::convert::TryInto;

// RANDOM FIELD ELEMENT GENERATOR
// ================================================================================================

pub struct RandomElementGenerator {
    seed: [u8; 64],
    hash_fn: HashFunction,
}

impl RandomElementGenerator {
    pub fn new(seed: [u8; 32], counter: u64, hash_fn: HashFunction) -> Self {
        let mut generator = RandomElementGenerator {
            seed: [0u8; 64],
            hash_fn,
        };
        generator.seed[..32].copy_from_slice(&seed);
        generator.seed[56..].copy_from_slice(&counter.to_le_bytes());
        generator
    }

    /// Returns little-ending representation of the value stored in the last 8 bytes of the seed.
    pub fn counter(&self) -> u64 {
        u64::from_le_bytes(self.seed[56..].try_into().unwrap())
    }

    // DRAW METHODS
    // --------------------------------------------------------------------------------------------

    /// Generates the next pseudo-random field element.
    pub fn draw<E: FieldElement>(&mut self) -> E {
        let hash = self.hash_fn;
        let mut result = [0u8; 32];
        loop {
            // updated the seed by incrementing its counter and then hash the result
            self.increment_counter();
            hash(&self.seed, &mut result);

            // take the first ELEMENT_BYTES from the hashed seed and check if they can be converted
            // into a valid field element; if the can, return; otherwise try again
            if let Some(element) = E::from_random_bytes(&result[..(E::ELEMENT_BYTES as usize)]) {
                return element;
            }
        }
    }

    /// Generates the next pair of pseudo-random field element.
    pub fn draw_pair<E: FieldElement>(&mut self) -> (E, E) {
        (self.draw(), self.draw())
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    /// Update the seed by incrementing the value in the last 8 bytes by 1.
    fn increment_counter(&mut self) {
        let mut counter = u64::from_le_bytes(self.seed[56..].try_into().unwrap());
        counter += 1;
        self.seed[56..].copy_from_slice(&counter.to_le_bytes());
    }
}
