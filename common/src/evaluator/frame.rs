use math::field::FieldElement;

pub struct EvaluationFrame<E: FieldElement> {
    pub current: Vec<E>,
    pub next: Vec<E>,
}

impl<E: FieldElement> EvaluationFrame<E> {
    pub fn new(num_registers: usize) -> Self {
        EvaluationFrame {
            current: E::zeroed_vector(num_registers),
            next: E::zeroed_vector(num_registers),
        }
    }
}
