use crypto::HashFunction;
use math::field::{FieldElement, StarkField};
use std::convert::TryInto;

pub struct RandomGenerator {
    seed: [u8; 64],
    hash_fn: HashFunction,
}

impl RandomGenerator {
    pub fn new(seed: [u8; 32], counter: u64, hash_fn: HashFunction) -> Self {
        let mut generator = RandomGenerator {
            seed: [0u8; 64],
            hash_fn,
        };
        generator.seed[..32].copy_from_slice(&seed);
        generator.seed[56..].copy_from_slice(&counter.to_le_bytes());
        generator
    }

    /// Generates the next pseudo-random field element.
    /// TODO: verify that this method of drawing random field elements is OK.
    pub fn draw(&mut self) -> FieldElement {
        let hash = self.hash_fn;
        let mut result = [0u8; 32];
        loop {
            // update the seed by incrementing the value in the last 8 bytes by 1
            let mut counter = u64::from_le_bytes(self.seed[56..].try_into().unwrap());
            counter += 1;
            self.seed[56..].copy_from_slice(&counter.to_le_bytes());

            // hash the seed
            hash(&self.seed, &mut result);

            // take the first MODULUS_BYTES from the hashed seed and check if they can be converted
            // into a valid field element; if the can, return; otherwise try again
            if let Some(element) =
                FieldElement::from_random_bytes(&result[..(FieldElement::MODULUS_BYTES as usize)])
            {
                return element;
            }
        }
    }

    /// Generates the next pair of pseudo-random field element.
    pub fn draw_pair(&mut self) -> (FieldElement, FieldElement) {
        (self.draw(), self.draw())
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {

    use super::FieldElement;
    use crypto::hash::blake3;

    #[test]
    fn draw() {
        let mut generator = super::RandomGenerator::new([0; 32], 0, blake3);

        let result = generator.draw();
        assert_eq!(
            result,
            FieldElement::new(257367016314067561345826246336977956381)
        );

        let result = generator.draw();
        assert_eq!(
            result,
            FieldElement::new(71356866342624880993791800984977673254)
        );

        let result = generator.draw();
        assert_eq!(
            result,
            FieldElement::new(209866678167327876517963759170433911820)
        );
    }
}
