use crate::{folding::quartic, utils, FriProof, FriProofLayer, ProverChannel};
use crypto::{hash, BatchMerkleProof, HashFunction, MerkleTree};
use kompact::prelude::*;
use math::field::{BaseElement, FieldElement};
use std::{marker::PhantomData, sync::Arc};

mod manager;
use manager::Manager;

mod messages;
use messages::{ManagerMessage, QueryResult};

mod worker;

#[cfg(test)]
mod tests;

// FRI PROVER
// ================================================================================================

pub struct FriProver<C: ProverChannel> {
    manager: Arc<Component<Manager>>,
    manager_ref: ActorRefStrong<ManagerMessage>,
    num_workers: usize,
    folding_factor: usize,
    max_remainder_length: usize,
    hash_fn: HashFunction,
    request: Option<ProofRequest>,
    _marker: PhantomData<C>,
}

impl<C: ProverChannel> FriProver<C> {
    pub fn new(system: &KompactSystem, num_workers: usize) -> Self {
        let manager = system.create(move || Manager::new(num_workers));
        system.start(&manager);
        let manager_ref = manager.actor_ref().hold().expect("live");

        FriProver {
            manager,
            manager_ref,
            num_workers,
            folding_factor: 4,
            max_remainder_length: 256,
            hash_fn: hash::blake3,
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

        // distribute evaluations
        let evaluations = Arc::new(evaluations.to_vec());
        self.manager_ref
            .ask(|promise| ManagerMessage::DistributeEvaluations(Ask::new(promise, evaluations)))
            .wait();

        // determine number of layers
        let num_layers =
            get_num_layers(domain_size, self.folding_factor, self.max_remainder_length);

        let mut layer_trees = Vec::new();
        for layer_depth in 0..num_layers {
            // commit to the current layer across all workers; we do this by first having each
            // worker commit to their current layers, and then building a Merkle tree from
            // worker commitments.
            let worker_commitments = self
                .manager_ref
                .ask(|promise| ManagerMessage::CommitToLayer(Ask::new(promise, ())))
                .wait();
            let layer_tree = MerkleTree::new(worker_commitments, self.hash_fn);
            channel.commit_fri_layer(*layer_tree.root());
            layer_trees.push(layer_tree);

            // draw random coefficient from the channel and use it to perform degree-respecting
            // projections in each worker.
            let alpha = channel.draw_fri_alpha::<BaseElement>(layer_depth);
            self.manager_ref
                .ask(|promise| ManagerMessage::ApplyDrp(Ask::new(promise, alpha)))
                .wait();
        }

        // retrieve remainder and commit to it
        let remainder = self
            .manager_ref
            .ask(|promise| ManagerMessage::RetrieveRemainder(Ask::new(promise, ())))
            .wait();
        let remainder_folded = quartic::transpose(&remainder, 1);
        let remainder_hashes = utils::hash_values(&remainder_folded, self.hash_fn);
        let remainder_tree = MerkleTree::new(remainder_hashes, self.hash_fn);
        channel.commit_fri_layer(*remainder_tree.root());

        // build and set active request
        self.request = Some(ProofRequest {
            domain_size,
            folding_factor: self.folding_factor,
            num_partitions: self.num_workers,
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
            .ask(|promise| ManagerMessage::QueryLayers(Ask::new(promise, positions.to_vec())))
            .wait();

        // iterate over all queried partitions
        for (partition_idx, partition_results) in partition_queries.into_iter() {
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
        let layers = queries
            .into_iter()
            .map(|mut q| build_fri_layer(&mut q))
            .collect();

        FriProof {
            layers,
            rem_values: BaseElement::write_into_vec(&request.remainder),
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

// HELPER FUNCTIONS
// ================================================================================================
fn get_num_layers(
    mut domain_size: usize,
    folding_factor: usize,
    max_remainder_length: usize,
) -> usize {
    let mut result = 0;
    while domain_size > max_remainder_length {
        domain_size /= folding_factor;
        result += 1;
    }
    result
}

fn build_fri_layer(queries: &mut [QueryResult]) -> FriProofLayer {
    queries.sort_by_key(|q| q.index);

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
