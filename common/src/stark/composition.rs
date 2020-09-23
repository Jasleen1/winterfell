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
