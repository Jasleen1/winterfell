use math::field::FieldElement;

pub trait ProverChannel {
    fn commit_fri_layer(&mut self, layer_root: [u8; 32]);

    fn draw_fri_point<E: FieldElement>(&self, layer_idx: usize) -> E;
}
