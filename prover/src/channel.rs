use common::{
    proof::{Commitments, Context, FriProof, OodEvaluationFrame, Queries, StarkProof},
    ComputationContext, EvaluationFrame, PublicCoin,
};
use crypto::{BatchMerkleProof, HashFunction};
use math::field::{BaseElement, FieldElement, StarkField};
use std::convert::TryInto;

#[cfg(not(feature = "extension_field"))]
const FIELD_EXTENSION_FACTOR: u32 = 1;
#[cfg(feature = "extension_field")]
const FIELD_EXTENSION_FACTOR: u32 = 2;

// TYPES AND INTERFACES
// ================================================================================================

pub struct ProverChannel {
    context: ComputationContext,
    trace_root: Option<[u8; 32]>,
    constraint_root: Option<[u8; 32]>,
    fri_roots: Vec<[u8; 32]>,
    query_seed: Option<[u8; 32]>,
    pow_nonce: u64,
}

// PROVER CHANNEL IMPLEMENTATION
// ================================================================================================

impl ProverChannel {
    /// Creates a new prover channel for the specified proof `context`.
    pub fn new(context: &ComputationContext) -> Self {
        ProverChannel {
            context: context.clone(),
            trace_root: None,
            constraint_root: None,
            fri_roots: Vec::new(),
            query_seed: None,
            pow_nonce: 0,
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

    /// Computes query seed from a combination of FRI layers and applies PoW to the seed
    /// based on the grinding_factor specified by the options
    pub fn grind_query_seed(&mut self) {
        assert!(
            !self.fri_roots.is_empty(),
            "FRI layers haven't been computed yet"
        );
        assert!(
            self.query_seed.is_none(),
            "query seed has already been computed"
        );
        let options = self.context().options();
        let seed = build_query_seed(&self.fri_roots, options.hash_fn());
        let (seed, nonce) = find_pow_nonce(seed, options.grinding_factor(), options.hash_fn());
        self.query_seed = Some(seed);
        self.pow_nonce = nonce;
    }

    /// Builds a proof from the previously committed values as well as values
    /// passed in to this method
    pub fn build_proof<E: FieldElement>(
        self,
        trace_paths: BatchMerkleProof,
        trace_states: Vec<Vec<BaseElement>>,
        constraint_paths: BatchMerkleProof,
        ood_frame: EvaluationFrame<E>,
        fri_proof: FriProof,
    ) -> StarkProof {
        StarkProof {
            context: Context {
                trace_width: self.context.trace_width() as u8,
                lde_domain_depth: trace_paths.depth,
                ce_blowup_factor: self.context.ce_blowup_factor() as u8,
                field_modulus_bytes: BaseElement::MODULUS.to_be_bytes().to_vec(),
                field_extension_factor: FIELD_EXTENSION_FACTOR,
                options: self.context().options().clone(),
            },
            commitments: Commitments {
                trace_root: self.trace_root.unwrap(),
                constraint_root: self.constraint_root.unwrap(),
                fri_roots: self.fri_roots,
            },
            queries: Queries {
                trace_paths: trace_paths.nodes,
                trace_states: trace_states
                    .into_iter()
                    .map(|s| BaseElement::write_into_vec(&s))
                    .collect(),
                constraint_proof: constraint_paths,
            },
            ood_frame: OodEvaluationFrame {
                trace_at_z1: E::write_into_vec(&ood_frame.current),
                trace_at_z2: E::write_into_vec(&ood_frame.next),
            },
            fri_proof,
            pow_nonce: self.pow_nonce,
        }
    }
}

// PUBLIC COIN IMPLEMENTATION
// ================================================================================================

impl PublicCoin for ProverChannel {
    fn context(&self) -> &ComputationContext {
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
        assert!(self.query_seed.is_some(), "query seed is not set");
        self.query_seed.unwrap()
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

    query_seed
}

fn find_pow_nonce(seed: [u8; 32], grinding_factor: u32, hash: HashFunction) -> ([u8; 32], u64) {
    let mut buf = [0u8; 64];
    let mut result = [0u8; 32];
    let mut nonce = 1u64;

    // copy seed into the buffer
    buf[0..32].copy_from_slice(&seed);
    // initial round
    buf[56..].copy_from_slice(&nonce.to_le_bytes());
    hash(&buf, &mut result);

    // increment the counter (u64 in the last 8 bytes) and hash until the output starts
    // with the number of leading zeros specified by the grinding_factor
    while u64::from_le_bytes(result[..8].try_into().unwrap()).trailing_zeros() < grinding_factor {
        nonce += 1;
        buf[56..].copy_from_slice(&nonce.to_le_bytes());

        hash(&buf, &mut result);
    }

    (result, nonce)
}
