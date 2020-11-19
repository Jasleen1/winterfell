use math::field::FieldElement;
pub struct EvaluationFrame<E: FieldElement> {
    pub current: Vec<E>,
    pub next: Vec<E>,
}
