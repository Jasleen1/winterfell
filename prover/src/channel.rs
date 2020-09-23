use common::stark::{ProofContext, PublicCoin};

pub struct ProverChannel {
    context: ProofContext,
    constraint_seed: Option<[u8; 32]>,
    composition_seed: Option<[u8; 32]>,
    query_seed: Option<[u8; 32]>,
}

impl ProverChannel {
    pub fn new(context: &ProofContext) -> Self {
        ProverChannel {
            context: context.clone(),
            constraint_seed: None,
            composition_seed: None,
            query_seed: None,
        }
    }

    pub fn commit_trace(&mut self, trace_root: [u8; 32]) {
        assert!(
            self.constraint_seed.is_none(),
            "trace root has already been committed"
        );
        self.constraint_seed = Some(trace_root);
    }

    pub fn commit_constraints(&mut self, constraint_root: [u8; 32]) {
        assert!(
            self.composition_seed.is_none(),
            "constraint root has already been committed"
        );
        self.composition_seed = Some(constraint_root);
    }

    pub fn commit_fri(&mut self, fri_roots: Vec<u8>) -> u64 {
        assert!(
            self.query_seed.is_none(),
            "fri_roots have already been committed"
        );
        let mut query_seed = [0u8; 32];
        self.context.options().hash_fn()(&fri_roots, &mut query_seed);
        let pow_nonce = 0;
        // TODO: let (query_seed, pow_nonce) = utils::find_pow_nonce(query_seed, &options);
        self.query_seed = Some(query_seed);
        pow_nonce
    }
}

impl PublicCoin for ProverChannel {
    fn context(&self) -> &ProofContext {
        &self.context
    }

    fn constraint_seed(&self) -> [u8; 32] {
        assert!(self.constraint_seed.is_some(), "constraint seed is not set");
        self.constraint_seed.unwrap()
    }

    fn composition_seed(&self) -> [u8; 32] {
        assert!(
            self.composition_seed.is_some(),
            "composition seed is not set"
        );
        self.composition_seed.unwrap()
    }

    fn query_seed(&self) -> [u8; 32] {
        assert!(self.query_seed.is_some(), "query seed is not set");
        self.query_seed.unwrap()
    }
}
