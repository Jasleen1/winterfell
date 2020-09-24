use super::{CompositionCoefficients, ProofContext};
use math::field;
use rand::distributions::Uniform;
use rand::prelude::*;

pub trait PublicCoin {
    // ABSTRACT METHODS
    // --------------------------------------------------------------------------------------------

    fn context(&self) -> &ProofContext;
    fn constraint_seed(&self) -> [u8; 32];
    fn composition_seed(&self) -> [u8; 32];
    fn fri_layer_seed(&self, layer_depth: usize) -> [u8; 32];
    fn query_seed(&self) -> [u8; 32];

    // DRAW METHODS
    // --------------------------------------------------------------------------------------------

    /// Draw coefficients for combining constraints using PRNG seeded with constraint seed.
    fn draw_constraint_coefficients(&self, num_coefficients: usize) -> Vec<u128> {
        field::prng_vector(self.constraint_seed(), num_coefficients)
    }

    /// Draws a point from the entire field using PRNG seeded with composition seed.
    fn draw_deep_point(&self) -> u128 {
        field::prng(self.composition_seed())
    }

    /// Draws coefficients for building composition polynomial using PRNG seeded with
    /// composition seed.
    fn draw_composition_coefficients(&self) -> CompositionCoefficients {
        let mut prng = field::prng_iter(self.composition_seed());
        prng.next().unwrap(); // skip z
        CompositionCoefficients::new(&mut prng, self.context().trace_width())
    }

    fn draw_fri_point(&self, layer_depth: usize) -> u128 {
        field::prng(self.fri_layer_seed(layer_depth))
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
