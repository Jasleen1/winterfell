use crate::ComputationContext;
use crypto::HashFunction;
use math::field::{FieldElement, StarkField};
use rand::distributions::Uniform;
use rand::prelude::*;
use std::convert::TryInto;

#[cfg(test)]
mod tests;

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

    // DRAW METHODS
    // --------------------------------------------------------------------------------------------

    /// Draws a point from the entire field using PRNG seeded with composition seed.
    fn draw_deep_point(&self) -> FieldElement {
        FieldElement::prng_vector(self.composition_seed(), 1)[0]
    }

    /// Draws coefficients for building composition polynomial using PRNG seeded with
    /// composition seed.
    fn draw_composition_coefficients(&self) -> CompositionCoefficients {
        let mut prng = FieldElement::prng_iter(self.composition_seed());
        prng.next().unwrap(); // skip z
        CompositionCoefficients::new(&mut prng, self.context().trace_width())
    }

    fn draw_fri_point(&self, layer_depth: usize) -> FieldElement {
        FieldElement::prng_vector(self.fri_layer_seed(layer_depth), 1)[0]
    }

    /// Draws a set of unique query positions using PRNG seeded with query seed. The positions
    /// are selected from the range [0..lde_domain_size], and all multiples of blowup factor
    /// are skipped.
    fn draw_query_positions(&self) -> Vec<usize> {
        let num_queries = self.context().options().num_queries();
        let lde_blowup_factor = self.context().options().blowup_factor();

        // use the seed to construct a PRNG
        let range = Uniform::from(0..self.context().lde_domain_size());
        let mut index_iter = StdRng::from_seed(self.query_seed()).sample_iter(range);

        // draw values from PRNG until we get as many unique values as specified by
        // num_queries, but skipping values which are a multiple of blowup factor
        let mut result = Vec::new();
        for _ in 0..1000 {
            let value = index_iter.next().unwrap();
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

// COMPOSITION COEFFICIENTS
// ================================================================================================

pub struct CompositionCoefficients {
    pub trace1: Vec<FieldElement>,
    pub trace2: Vec<FieldElement>,
    pub t1_degree: FieldElement,
    pub t2_degree: FieldElement,
    pub constraints: FieldElement,
}

impl CompositionCoefficients {
    pub fn new<T: Iterator<Item = u128>>(prng: &mut T, trace_width: usize) -> Self {
        CompositionCoefficients {
            trace1: prng.take(2 * trace_width).map(FieldElement::from).collect(),
            trace2: prng.take(2 * trace_width).map(FieldElement::from).collect(),
            t1_degree: FieldElement::from(prng.next().unwrap()),
            t2_degree: FieldElement::from(prng.next().unwrap()),
            constraints: FieldElement::from(prng.next().unwrap()),
        }
    }
}
