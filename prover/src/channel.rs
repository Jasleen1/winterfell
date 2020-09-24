use common::stark::{
    Commitments, Context, DeepValues, FriProof, ProofContext, PublicCoin, Queries, StarkProof,
};
use crypto::{BatchMerkleProof, HashFunction};

// TYPES AND INTERFACES
// ================================================================================================

pub struct ProverChannel {
    context: ProofContext,
    trace_root: Option<[u8; 32]>,
    constraint_root: Option<[u8; 32]>,
    fri_roots: Vec<[u8; 32]>,
}

// PROVER CHANNEL IMPLEMENTATION
// ================================================================================================

impl ProverChannel {
    /// Creates a new prover channel for the specified proof `context`.
    pub fn new(context: &ProofContext) -> Self {
        ProverChannel {
            context: context.clone(),
            trace_root: None,
            constraint_root: None,
            fri_roots: Vec::new(),
        }
    }

    /// Commits the prover the extended execution trace.
    pub fn commit_trace(&mut self, trace_root: [u8; 32]) {
        assert!(
            self.trace_root.is_none(),
            "trace root has already been committed"
        );
        self.trace_root = Some(trace_root);
    }

    /// Commits the prover the the constraint evaluations.
    pub fn commit_constraints(&mut self, constraint_root: [u8; 32]) {
        assert!(
            self.constraint_root.is_none(),
            "constraint root has already been committed"
        );
        self.constraint_root = Some(constraint_root);
    }

    /// Commits the prover to the a FRI layer.
    pub fn commit_fri_layer(&mut self, layer_root: [u8; 32]) {
        self.fri_roots.push(layer_root);
    }

    /// Builds a proof from the previously committed values as well as values
    /// passed in to this method
    pub fn build_proof(
        self,
        trace_paths: BatchMerkleProof,
        trace_states: Vec<Vec<u128>>,
        constraint_paths: BatchMerkleProof,
        deep_values: DeepValues,
        fri_proof: FriProof,
    ) -> StarkProof {
        StarkProof {
            context: Context {
                lde_domain_depth: trace_paths.depth,
                max_constraint_degree: self.context.max_constraint_degree() as u8,
                options: self.context().options().clone(),
            },
            commitments: Commitments {
                trace_root: self.trace_root.unwrap(),
                constraint_root: self.constraint_root.unwrap(),
                fri_roots: self.fri_roots,
            },
            queries: Queries {
                trace_paths: trace_paths.nodes,
                trace_states,
                constraint_proof: constraint_paths,
            },
            deep_values,
            fri_proof,
            pow_nonce: 0,
        }
    }
}

// PUBLIC COIN IMPLEMENTATION
// ================================================================================================

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

    fn fri_layer_seed(&self, layer_depth: usize) -> [u8; 32] {
        assert!(!self.fri_roots.is_empty(), "FRI layers are not set");
        self.fri_roots[layer_depth]
    }

    fn query_seed(&self) -> [u8; 32] {
        assert!(!self.fri_roots.is_empty(), "query seed is not set");
        build_query_seed(&self.fri_roots, self.context.options().hash_fn())
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_query_seed(fri_roots: &[[u8; 32]], hash_fn: HashFunction) -> [u8; 32] {
    // combine roots of all FIR layers into a single array of bytes
    let mut root_bytes: Vec<u8> = Vec::with_capacity(fri_roots.len() * 32);
    for root in fri_roots.iter() {
        root.iter().for_each(|&v| root_bytes.push(v));
    }

    // hash the array of bytes into a single 32-byte value
    let mut query_seed = [0u8; 32];
    hash_fn(&root_bytes, &mut query_seed);

    // TODO: let (query_seed, pow_nonce) = utils::find_pow_nonce(query_seed, &options);
    query_seed
}
