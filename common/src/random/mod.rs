use crate::ComputationContext;
use crypto::HashFunction;
use math::field::{FieldElement, StarkField};
use std::{convert::TryInto, mem::size_of};

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================
const TRANSITION_COEFF_OFFSET: u64 = 0;
const ASSERTION_COEFF_OFFSET: u64 = u32::MAX as u64;
const DEEP_POINT_OFFSET: u64 = 0;
const COMPOSITION_COEFF_OFFSET: u64 = 1024;
const FRI_POINT_OFFSET: u64 = 0;

// PUBLIC COIN
// ================================================================================================

pub trait PublicCoin {
    // ABSTRACT METHODS
    // --------------------------------------------------------------------------------------------

    fn context(&self) -> &ComputationContext;
    fn constraint_seed(&self) -> [u8; 32];
    fn composition_seed(&self) -> [u8; 32];
    fn fri_layer_seed(&self, layer_depth: usize) -> [u8; 32];
    fn query_seed(&self) -> [u8; 32];

    // PRNG BUILDERS
    // --------------------------------------------------------------------------------------------

    /// Returns a PRNG for transition constraint coefficients.
    fn get_transition_coefficient_prng(&self) -> RandomGenerator {
        RandomGenerator::new(
            self.constraint_seed(),
            TRANSITION_COEFF_OFFSET,
            self.context().options().hash_fn(),
        )
    }

    /// Returns a PRNG for assertion constraint coefficients.
    fn get_assertion_coefficient_prng(&self) -> RandomGenerator {
        RandomGenerator::new(
            self.constraint_seed(),
            ASSERTION_COEFF_OFFSET,
            self.context().options().hash_fn(),
        )
    }

    // DRAW METHODS
    // --------------------------------------------------------------------------------------------

    /// Draws a point from the entire field using PRNG seeded with composition seed.
    fn draw_deep_point(&self) -> FieldElement {
        let mut generator = RandomGenerator::new(
            self.composition_seed(),
            DEEP_POINT_OFFSET,
            self.context().options().hash_fn(),
        );
        let result = generator.draw();
        assert!(
            generator.counter() < COMPOSITION_COEFF_OFFSET,
            "drawing DEEP point required {} tries",
            generator.counter()
        );
        result
    }

    /// Draws coefficients for building composition polynomial using PRNG seeded with
    /// composition seed.
    fn draw_composition_coefficients(&self) -> CompositionCoefficients {
        let generator = RandomGenerator::new(
            self.composition_seed(),
            COMPOSITION_COEFF_OFFSET,
            self.context().options().hash_fn(),
        );
        CompositionCoefficients::new(generator, self.context().trace_width())
    }

    fn draw_fri_point(&self, layer_depth: usize) -> FieldElement {
        let mut generator = RandomGenerator::new(
            self.fri_layer_seed(layer_depth),
            FRI_POINT_OFFSET,
            self.context().options().hash_fn(),
        );
        generator.draw()
    }

    /// Draws a set of unique query positions using PRNG seeded with query seed. The positions
    /// are selected from the range [0..lde_domain_size], and all multiples of blowup factor
    /// are skipped.
    /// TODO: verify that the distribution of query positions is actually uniformly random
    fn draw_query_positions(&self) -> Vec<usize> {
        let hash = self.context().options().hash_fn();
        let num_queries = self.context().options().num_queries();
        let lde_blowup_factor = self.context().options().blowup_factor();

        // determine how many bits are needed to represent valid indexes in the domain
        let value_mask = self.context().lde_domain_size() - 1;
        let value_offset = 32 - size_of::<usize>();

        // initialize the seed for PRNG
        let mut seed = [0u8; 64];
        seed[..32].copy_from_slice(&self.query_seed());
        let mut value_bytes = [0u8; 32];

        // draw values from PRNG until we get as many unique values as specified by
        // num_queries, but skipping values which are a multiple of blowup factor
        let mut result = Vec::new();
        for i in 0usize..1000 {
            // update the seed with the new counter and hash the result
            seed[56..].copy_from_slice(&i.to_le_bytes());
            hash(&seed, &mut value_bytes);

            // read the required number of bits from the hashed value
            let value =
                usize::from_le_bytes(value_bytes[value_offset..].try_into().unwrap()) & value_mask;

            if value % lde_blowup_factor == 0 {
                continue;
            }
            if result.contains(&value) {
                continue;
            }
            result.push(value);
            if result.len() >= num_queries {
                break;
            }
        }

        assert!(
            result.len() == num_queries,
            "needed to generate {} query positions, but generated only {}",
            num_queries,
            result.len()
        );

        result
    }
}

// COMPOSITION COEFFICIENTS
// ================================================================================================

#[derive(Debug)]
pub struct CompositionCoefficients {
    pub trace: Vec<(FieldElement, FieldElement)>,
    pub trace_degree: (FieldElement, FieldElement),
    pub constraints: FieldElement,
}

impl CompositionCoefficients {
    pub fn new(mut prng: RandomGenerator, trace_width: usize) -> Self {
        CompositionCoefficients {
            trace: (0..trace_width).map(|_| prng.draw_pair()).collect(),
            trace_degree: prng.draw_pair(),
            constraints: prng.draw(),
        }
    }
}

// RANDOM ELEMENT GENERATOR
// ================================================================================================

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

    /// Returns little-ending representation of the value stored in the last 8 bytes of the seed.
    pub fn counter(&self) -> u64 {
        u64::from_le_bytes(self.seed[56..].try_into().unwrap())
    }

    // DRAW METHODS
    // --------------------------------------------------------------------------------------------

    /// Generates the next pseudo-random field element.
    /// TODO: verify that this method of drawing random field elements is OK.
    pub fn draw(&mut self) -> FieldElement {
        let hash = self.hash_fn;
        let mut result = [0u8; 32];
        loop {
            // updated the seed by incrementing its counter and then hash the result
            self.increment_counter();
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

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    /// Update the seed by incrementing the value in the last 8 bytes by 1.
    fn increment_counter(&mut self) {
        let mut counter = u64::from_le_bytes(self.seed[56..].try_into().unwrap());
        counter += 1;
        self.seed[56..].copy_from_slice(&counter.to_le_bytes());
    }
}
