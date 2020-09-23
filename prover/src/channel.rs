use common::stark::{DeepValues, FriProof, ProofContext, PublicCoin, StarkProof};
use crypto::BatchMerkleProof;

pub struct ProverChannel {
    context: ProofContext,
    trace_root: Option<[u8; 32]>,
    constraint_root: Option<[u8; 32]>,
    query_seed: Option<[u8; 32]>,
}

impl ProverChannel {
    pub fn new(context: &ProofContext) -> Self {
        ProverChannel {
            context: context.clone(),
            trace_root: None,
            constraint_root: None,
            query_seed: None,
        }
    }

    pub fn commit_trace(&mut self, trace_root: [u8; 32]) {
        assert!(
            self.trace_root.is_none(),
            "trace root has already been committed"
        );
        self.trace_root = Some(trace_root);
    }

    pub fn commit_constraints(&mut self, constraint_root: [u8; 32]) {
        assert!(
            self.constraint_root.is_none(),
            "constraint root has already been committed"
        );
        self.constraint_root = Some(constraint_root);
    }

    pub fn commit_fri(&mut self, fri_roots: Vec<u8>) {
        assert!(
            self.query_seed.is_none(),
            "fri_roots have already been committed"
        );
        let mut query_seed = [0u8; 32];
        self.context.options().hash_fn()(&fri_roots, &mut query_seed);
        // TODO: let (query_seed, pow_nonce) = utils::find_pow_nonce(query_seed, &options);
        self.query_seed = Some(query_seed);
    }

    pub fn build_proof(
        self,
        trace_paths: BatchMerkleProof,
        trace_states: Vec<Vec<u128>>,
        constraint_paths: BatchMerkleProof,
        deep_values: DeepValues,
        fri_proof: FriProof,
    ) -> StarkProof {
        StarkProof {
            trace_root: self.trace_root.unwrap(),
            lde_domain_depth: trace_paths.depth,
            trace_nodes: trace_paths.nodes,
            trace_states,
            constraint_root: self.constraint_root.unwrap(),
            constraint_proof: constraint_paths,
            max_constraint_degree: self.context.max_constraint_degree() as u8,
            deep_values,
            fri_proof,
            pow_nonce: 0,
            options: self.context().options().clone(),
        }
    }
}

impl PublicCoin for ProverChannel {
    fn context(&self) -> &ProofContext {
        &self.context
    }

    fn constraint_seed(&self) -> [u8; 32] {
        assert!(self.trace_root.is_some(), "constraint seed is not set");
        self.trace_root.unwrap()
    }

    fn composition_seed(&self) -> [u8; 32] {
        assert!(
            self.constraint_root.is_some(),
            "composition seed is not set"
        );
        self.constraint_root.unwrap()
    }

    fn query_seed(&self) -> [u8; 32] {
        assert!(self.query_seed.is_some(), "query seed is not set");
        self.query_seed.unwrap()
    }
}
