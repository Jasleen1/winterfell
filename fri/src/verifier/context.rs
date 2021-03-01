use crate::FriOptions;
use math::field::{BaseElement, StarkField};

pub struct VerifierContext {
    max_degree: usize,
    domain_size: usize,
    domain_root: BaseElement,
    options: FriOptions,
    num_partitions: usize,
}

impl VerifierContext {
    pub fn new(
        domain_size: usize,
        max_degree: usize,
        num_partitions: usize,
        options: FriOptions,
    ) -> Self {
        let domain_root = BaseElement::get_root_of_unity(domain_size.trailing_zeros());
        VerifierContext {
            max_degree,
            domain_size,
            domain_root,
            options,
            num_partitions,
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

    pub fn num_partitions(&self) -> usize {
        self.num_partitions
    }

    pub fn blowup_factor(&self) -> usize {
        self.options.blowup_factor()
    }

    pub fn folding_factor(&self) -> usize {
        self.options.folding_factor()
    }

    pub fn num_fri_layers(&self) -> usize {
        self.options.num_fri_layers(self.domain_size)
    }
}
