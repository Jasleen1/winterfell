use math::field;

pub trait ConstraintEvaluator {
    const MAX_CONSTRAINT_DEGREE: usize;

    fn new(
        seed: [u8; 32],
        trace_length: usize,
        blowup_factor: usize,
        assertions: Vec<usize>,
    ) -> Self;

    fn trace_length(&self) -> usize;
    fn domain_size(&self) -> usize;
    fn blowup_factor(&self) -> usize;

    fn evaluate_transition(
        &self,
        current: &Vec<u128>,
        next: &Vec<u128>,
        x: u128,
        step: usize,
    ) -> u128;

    fn evaluate_boundaries(&self, current: &Vec<u128>, x: u128) -> Vec<u128>;

    fn composition_degree(&self) -> usize {
        (Self::MAX_CONSTRAINT_DEGREE - 1) * self.trace_length() - 1
    }

    fn incremental_trace_degree(&self) -> usize {
        self.composition_degree() - (self.trace_length() - 2)
    }

    fn get_x_at(&self, step: usize) -> u128 {
        let trace_root = field::get_root_of_unity(self.trace_length());
        field::exp(trace_root, step as u128)
    }
}
