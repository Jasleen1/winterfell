use crate::FriOptions;
use math::field::{BaseElement, StarkField};

pub struct VerifierContext {
    max_degree: usize,
    domain_size: usize,
    domain_root: BaseElement,
    options: FriOptions,
}

impl VerifierContext {
    pub fn new(domain_size: usize, max_degree: usize, options: FriOptions) -> Self {
        let domain_root = BaseElement::get_root_of_unity(domain_size.trailing_zeros());
        VerifierContext {
            max_degree,
            domain_size,
            domain_root,
            options,
        }
    }

    pub fn max_degree(&self) -> usize {
        self.max_degree
    }

    pub fn domain_size(&self) -> usize {
        self.domain_size
    }

    pub fn domain_root(&self) -> BaseElement {
        self.domain_root
    }

    pub fn blowup_factor(&self) -> usize {
        self.options.blowup_factor()
    }

    pub fn num_fri_layers(&self) -> usize {
        let mut result = 0;
        let mut domain_size = self.domain_size;

        while domain_size > self.options.max_remainder_length() {
            domain_size /= self.options.folding_factor();
            result += 1;
        }

        result
    }
}
