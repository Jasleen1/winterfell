use math::field::FieldElement;

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
