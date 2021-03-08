use crate::{folding::quartic, utils, FriOptions, FriProof, FriProofLayer, ProverChannel};
use crypto::{BatchMerkleProof, MerkleTree};
use kompact::prelude::*;
use math::field::{BaseElement, FieldElement};
use std::{collections::HashMap, marker::PhantomData, sync::Arc, time::Duration};

mod manager;
use manager::Manager;

mod messages;
use fasthash::xx::Hash64;
use messages::{ProverRequest, QueryResult, RequestInfo};

mod worker;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================
const PROVER_PATH: &str = "fri_prover";
const SETTLE_TIME: Duration = Duration::from_millis(1000);

// FRI PROVER
// ================================================================================================

pub struct FriProver<C: ProverChannel> {
    options: FriOptions,
    manager_ref: ActorRefStrong<ProverRequest>,
    request: Option<ProofRequest>,
    _marker: PhantomData<C>,
}

impl<C: ProverChannel> FriProver<C> {
    /// Returns a new instance of the FRI prover instantiated with the specified `options`.
    /// This also starts a manager actor in the specified Kompact `system` and registers
    /// a named path to this actor at /fri_prover address.
    pub fn new(system: &KompactSystem, options: FriOptions) -> Self {
        // create a new manager actor and register it at the named path
        let (manager, manager_registration) = system.create_and_register(Manager::new);
        let manager_service_registration = system.register_by_alias(&manager, PROVER_PATH);

        // allow some time for registrations to settle
        let _manager_path =
            manager_registration.wait_expect(SETTLE_TIME, "failed to register manager");
        let _manager_named_path = manager_service_registration
            .wait_expect(SETTLE_TIME, "failed to register manager's named path");

        // start the manager and get a local reference to it
        system.start(&manager);
        let manager_ref = manager.actor_ref().hold().expect("live");

        FriProver {
            manager_ref,
            options,
            request: None,
            _marker: PhantomData,
        }
    }

    /// Executes commit phase of FRI protocol which recursively applies a degree-respecting projection
    /// to evaluations of some function F over a larger domain. At each layer of recursion the
    /// current evaluations are committed to using a Merkle tree, and the root of this tree is used
    /// to derive randomness for the subsequent application of degree-respecting projection.
    /// TODO: in a distributed context evaluations will need to be passed in as a set of data
    /// pointers (object IDs?) to data located on remote machines.
    pub fn build_layers(&mut self, channel: &mut C, evaluations: &[BaseElement]) {
        let domain_size = evaluations.len();
        let hash_fn = self.options.hash_fn();

        // distribute evaluations; number of partitions is set to the remainder length
        // so that each partitions resolves to a single value
        let num_layers = self.options.num_fri_layers(domain_size);
        let num_partitions = self.options.fri_remainder_length(domain_size);
        let evaluations = RequestInfo {
            evaluations: Arc::new(evaluations.to_vec()),
            num_partitions,
            num_layers,
        };
        self.manager_ref
            .ask(|promise| ProverRequest::InitRequest(Ask::new(promise, evaluations)))
            .wait();

        let mut layer_trees = Vec::new();
        for layer_depth in 0..num_layers {
            // commit to the current layer across all workers; we do this by first having each
            // worker commit to their current layers, and then building a Merkle tree from
            // worker commitments.
            let worker_commitments = self
                .manager_ref
                .ask(|promise| ProverRequest::CommitToLayer(Ask::new(promise, ())))
                .wait();
            let layer_tree = MerkleTree::new(worker_commitments, hash_fn);
            channel.commit_fri_layer(*layer_tree.root());
            layer_trees.push(layer_tree);

            // draw random coefficient from the channel and use it to perform degree-respecting
            // projections in each worker.
            let alpha = channel.draw_fri_alpha::<BaseElement>(layer_depth);
            self.manager_ref
                .ask(|promise| ProverRequest::ApplyDrp(Ask::new(promise, alpha)))
                .wait();
        }

        // retrieve remainder and commit to it
        let remainder = self
            .manager_ref
            .ask(|promise| ProverRequest::RetrieveRemainder(Ask::new(promise, ())))
            .wait();
        let remainder_folded = quartic::transpose(&remainder, 1);
        let remainder_hashes = quartic::hash_values(&remainder_folded, hash_fn);
        let remainder_tree = MerkleTree::new(remainder_hashes, hash_fn);
        channel.commit_fri_layer(*remainder_tree.root());

        // build and set active request
        self.request = Some(ProofRequest {
            domain_size,
            folding_factor: self.options.folding_factor(),
            num_partitions,
            layer_trees,
            remainder,
        });
    }

    /// Executes query phase of FRI protocol. For each of the provided `positions`, corresponding
    /// evaluations from each of the layers are recorded into the proof together with Merkle
    /// authentication paths from the root of layer commitment trees.
    pub fn build_proof(&mut self, positions: &[usize]) -> FriProof {
        let request = self.request.take().expect("request");
        let mut queries = (0..request.num_layers())
            .map(|_| Vec::new())
            .collect::<Vec<_>>();

        // get query results for all partitions from the manager
        let partition_queries = self
            .manager_ref
            .ask(|promise| ProverRequest::QueryLayers(Ask::new(promise, positions.to_vec())))
            .wait();

        // iterate over all queried partitions
        for (partition_idx, partition_results) in partition_queries.into_iter().enumerate() {
            // iterate over all layers within a partition result
            for (layer_depth, layer_results) in partition_results.into_iter().enumerate() {
                // queries for each partition contain only starting segments of Merkle
                // authentication paths against virtual Merkle tree of all evaluations -
                // so, we need to complete them with the segment held by the prover.
                let path_end = request.layer_trees[layer_depth].prove(partition_idx);
                for mut query in layer_results.into_iter() {
                    // empty path indicates that we reached the last layer where there was only
                    // a single leaf; in this case, we use the entire path from the prover.
                    if query.path.is_empty() {
                        query.path = path_end.clone();
                    } else {
                        // if we are not at the last layer, we need to skip the first node of
                        // the prover's path because it is implied by the partition path
                        query.path.extend_from_slice(&path_end[1..]);
                    }
                    // we also translate query index local to the partition into index applicable to
                    // the entire virtual tree
                    query.index = request.to_global_index(partition_idx, layer_depth, query.index);
                    queries[layer_depth].push(query);
                }
            }
        }

        // build FRI layers from the queries
        let folding_factor = request.folding_factor;
        let num_partitions = request.num_partitions;
        let mut positions = positions.to_vec();
        let mut domain_size = request.domain_size;
        let mut layers = Vec::new();
        for mut layer_queries in queries {
            positions = utils::fold_positions(&positions, domain_size, folding_factor);
            let indexes = utils::map_positions_to_indexes(
                &positions,
                domain_size,
                folding_factor,
                num_partitions,
            );
            layers.push(build_fri_layer(&mut layer_queries, indexes));
            domain_size /= request.folding_factor;
        }

        FriProof {
            layers,
            rem_values: BaseElement::write_into_vec(&request.remainder),
            partitioned: true,
        }
    }
}

// PROOF REQUEST
// ================================================================================================

struct ProofRequest {
    domain_size: usize,
    folding_factor: usize,
    num_partitions: usize,
    layer_trees: Vec<MerkleTree>,
    remainder: Vec<BaseElement>,
}

impl ProofRequest {
    pub fn num_layers(&self) -> usize {
        self.layer_trees.len()
    }

    pub fn to_global_index(
        &self,
        partition_idx: usize,
        layer_depth: usize,
        local_idx: usize,
    ) -> usize {
        let num_evaluations =
            self.domain_size / usize::pow(self.folding_factor, (layer_depth + 1) as u32);
        let local_bits = num_evaluations.trailing_zeros() - self.num_partitions.trailing_zeros();
        (partition_idx << local_bits) | local_idx
    }
}

// PROTOCOL STEP
// ================================================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolStep {
    None,
    Ready(usize),          // num_layers
    LayerCommitted(usize), // remaining _layers
    DrpApplied(usize),     // remaining layers
}

impl ProtocolStep {
    pub fn get_next_state(&self) -> ProtocolStep {
        match self {
            ProtocolStep::None => ProtocolStep::None,
            ProtocolStep::Ready(num_layers) => ProtocolStep::LayerCommitted(*num_layers),
            ProtocolStep::LayerCommitted(rem_layers) => ProtocolStep::DrpApplied(*rem_layers),
            ProtocolStep::DrpApplied(rem_layers) => {
                let rem_layers = *rem_layers;
                assert!(
                    rem_layers > 1,
                    "cannot apply degree-preserving projection to the last layer"
                );
                ProtocolStep::LayerCommitted(rem_layers - 1)
            }
        }
    }

    pub fn is_completed(&self) -> bool {
        if let ProtocolStep::DrpApplied(rem_layers) = self {
            *rem_layers == 1
        } else {
            false
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_fri_layer(queries: &mut [QueryResult], indexes: Vec<usize>) -> FriProofLayer {
    // make sure queries are sorted in exact same way as indexes
    let mut index_order_map = HashMap::with_hasher(Hash64);
    for (i, index) in indexes.into_iter().enumerate() {
        index_order_map.insert(index, i);
    }
    queries.sort_by(|a, b| {
        index_order_map
            .get(&a.index)
            .unwrap()
            .partial_cmp(index_order_map.get(&b.index).unwrap())
            .unwrap()
    });

    let mut indexes = Vec::new();
    let mut paths = Vec::new();
    let mut values = Vec::new();

    for query in queries.iter() {
        indexes.push(query.index);
        paths.push(query.path.clone());
        values.push(BaseElement::write_into_vec(&query.value));
    }

    let batch_proof = BatchMerkleProof::from_paths(&paths, &indexes);

    FriProofLayer {
        values,
        paths: batch_proof.nodes,
        depth: batch_proof.depth,
    }
}
