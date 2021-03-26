use math::field::FieldElement;

pub struct EvaluationFrame<E: FieldElement> {
    pub current: Vec<E>,
    pub next: Vec<E>,
}

impl<E: FieldElement> EvaluationFrame<E> {
    pub fn new(num_registers: usize) -> Self {
        EvaluationFrame {
            current: vec![E::ZERO; num_registers],
            next: vec![E::ZERO; num_registers],
        }
    }
}
