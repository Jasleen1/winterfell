use math::field;

/// Uses the provided `seed` to draw a random element from the entire field, and also
/// draws pseudo-random values which will be used as linear combination coefficients
/// for polynomial composition.
pub fn draw_z_and_coefficients(
    seed: [u8; 32],
    trace_width: usize,
) -> (u128, CompositionCoefficients) {
    let mut prng = field::prng_iter(seed);
    let z = prng.next().unwrap();
    let coefficients = CompositionCoefficients::new(&mut prng, trace_width);
    (z, coefficients)
}

pub struct CompositionCoefficients {
    pub trace1: Vec<u128>,
    pub trace2: Vec<u128>,
    pub t1_degree: u128,
    pub t2_degree: u128,
    pub constraints: u128,
}

impl CompositionCoefficients {
    pub fn new<T: Iterator<Item = u128>>(prng: &mut T, trace_width: usize) -> Self {
        CompositionCoefficients {
            trace1: prng.take(2 * trace_width).collect(),
            trace2: prng.take(2 * trace_width).collect(),
            t1_degree: prng.next().unwrap(),
            t2_degree: prng.next().unwrap(),
            constraints: prng.next().unwrap(),
        }
    }
}
